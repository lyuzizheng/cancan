//! macOS delivery for the opt-in background-intake notification.
//!
//! Everything macOS-specific about the feature lives here: the authorization
//! check, the banner request, and the delegate that turns a click back into
//! "reopen the window on this batch's Tasks row". The decision of *whether* a
//! batch is deliverable stays in [`super::notifications`].

use super::*;
use block2::DynBlock;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::{AllocAnyThread, DeclaredClass, define_class, msg_send};
use objc2_foundation::{NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent, UNNotification,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNNotificationSettings, UNNotificationSound, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::mpsc::{RecvTimeoutError, channel};

/// Long enough for the system to answer a permission or delivery callback, and
/// short enough that a wedged notification center cannot stall a pipeline pass.
const NOTIFICATION_CALLBACK_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct MacIntakeNotificationDelivery;

impl IntakeNotificationDelivery for MacIntakeNotificationDelivery {
    fn permission(&self) -> IntakeNotificationPermission {
        current_permission()
    }

    fn request_permission(&self) -> IntakeNotificationPermission {
        let center = user_notification_center();
        let (sender, receiver) = channel();
        let handler = block2::StackBlock::new(move |_granted: Bool, _error: *mut NSError| {
            let _ = sender.send(());
        });
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &handler,
        );
        // The system's own answer is the authority for what can be delivered,
        // so the granted flag only decides when to re-read it.
        if receiver.recv_timeout(NOTIFICATION_CALLBACK_TIMEOUT) == Err(RecvTimeoutError::Timeout) {
            eprintln!("intake notification permission request timed out");
        }
        current_permission()
    }

    fn deliver(&self, notification: &IntakeNotification) -> Result<(), RuntimeError> {
        let center = user_notification_center();
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&notification.title));
        content.setBody(&NSString::from_str(&notification.body));
        content.setSound(Some(&UNNotificationSound::defaultSound()));
        let identifier = NSString::from_str(&notification.identifier);
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &identifier,
            &content,
            None,
        );
        let (sender, receiver) = channel();
        let handler = block2::StackBlock::new(move |error: *mut NSError| {
            let _ = sender.send(error.is_null());
        });
        center.addNotificationRequest_withCompletionHandler(&request, Some(&handler));
        match receiver.recv_timeout(NOTIFICATION_CALLBACK_TIMEOUT) {
            Ok(true) => Ok(()),
            Ok(false) => Err(RuntimeError::new("intake_notification_rejected")),
            Err(_) => Err(RuntimeError::new("intake_notification_timed_out")),
        }
    }
}

/// Registers the click handler. The center holds its delegate weakly, so the
/// strong reference lives here for the life of the process, and a second call
/// keeps the first handler instead of replacing it.
pub(super) fn install(app: &AppHandle, runtime: &VaultRuntime) {
    if INTAKE_NOTIFICATION_CLICK_HANDLER.get().is_some() {
        return;
    }
    let center = user_notification_center();
    let handler = IntakeNotificationClickHandler::new(app.clone(), runtime.clone());
    center.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*handler)));
    if INTAKE_NOTIFICATION_CLICK_HANDLER
        .set(Mutex::new(handler))
        .is_err()
    {
        eprintln!("intake notification click handler was already installed");
    }
}

/// The notification center can only be used from a bundled application; an
/// unbundled development binary answers "not determined" instead of throwing.
/// Delivery is impossible there, so a batch keeps its pending state and the
/// next pass retries once the real app is running.
fn user_notification_center() -> Retained<UNUserNotificationCenter> {
    UNUserNotificationCenter::currentNotificationCenter()
}

fn is_bundled() -> bool {
    NSBundle::mainBundle().bundleIdentifier().is_some()
}

fn current_permission() -> IntakeNotificationPermission {
    if !is_bundled() {
        return IntakeNotificationPermission::NotDetermined;
    }
    let center = user_notification_center();
    let (sender, receiver) = channel();
    let handler = block2::StackBlock::new(move |settings: NonNull<UNNotificationSettings>| {
        // SAFETY: the center owns the settings object for the duration of the
        // callback, and the callback is called exactly once.
        let status = unsafe { settings.as_ref() }.authorizationStatus();
        let _ = sender.send(status);
    });
    center.getNotificationSettingsWithCompletionHandler(&handler);
    match receiver.recv_timeout(NOTIFICATION_CALLBACK_TIMEOUT) {
        Ok(status) if status == UNAuthorizationStatus::Authorized => {
            IntakeNotificationPermission::Authorized
        }
        Ok(status) if status == UNAuthorizationStatus::Denied => {
            IntakeNotificationPermission::Denied
        }
        // Not determined, provisional, or an answer that never arrived: nothing
        // is deliverable yet, and the setting stays on for the moment the user
        // decides.
        Ok(_) | Err(_) => IntakeNotificationPermission::NotDetermined,
    }
}

struct IntakeNotificationClickHandlerIvars {
    app: AppHandle,
    runtime: VaultRuntime,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this class does not
    // implement Drop.
    #[unsafe(super(NSObject))]
    #[name = "CanCanIntakeNotificationClickHandler"]
    #[ivars = IntakeNotificationClickHandlerIvars]
    struct IntakeNotificationClickHandler;

    impl IntakeNotificationClickHandler {
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive_notification_response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &DynBlock<dyn Fn()>,
        ) {
            let identifier = response.notification().request().identifier().to_string();
            if let Some(batch_id) = identifier.strip_prefix(INTAKE_NOTIFICATION_IDENTIFIER_PREFIX) {
                self.ivars().runtime.record_background_intake_route(batch_id);
            }
            // The window the click rebuilds and the route it lands on are the
            // same pair the menu-bar item opens, so both go through one path.
            let app = self.ivars().app.clone();
            let reopened_app = app.clone();
            let opened = app.run_on_main_thread(move || {
                if let Err(error) = show_or_create_main_window(&reopened_app) {
                    eprintln!("intake notification reopen failed: {error}");
                }
            });
            if let Err(error) = opened {
                eprintln!("intake notification reopen dispatch failed: {error}");
            }
            completion_handler.call(());
        }

        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present_notification(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            // A background-intake notification is only ever posted with no
            // CanCan window on screen, but the application can still be the
            // active one; ask for the banner and the sound explicitly instead
            // of letting the foreground default swallow it.
            completion_handler.call((
                UNNotificationPresentationOptions::Banner | UNNotificationPresentationOptions::Sound,
            ));
        }
    }

    unsafe impl NSObjectProtocol for IntakeNotificationClickHandler {}

    unsafe impl UNUserNotificationCenterDelegate for IntakeNotificationClickHandler {}
);

impl IntakeNotificationClickHandler {
    fn new(app: AppHandle, runtime: VaultRuntime) -> Retained<Self> {
        let this = Self::alloc().set_ivars(IntakeNotificationClickHandlerIvars { app, runtime });
        // SAFETY: NSObject's `init` takes an allocated instance of a subclass
        // whose ivars are already set.
        unsafe { msg_send![super(this), init] }
    }
}

/// The center keeps its delegate weakly, so the strong reference has to outlive
/// every delivery.
static INTAKE_NOTIFICATION_CLICK_HANDLER: OnceLock<
    Mutex<Retained<IntakeNotificationClickHandler>>,
> = OnceLock::new();

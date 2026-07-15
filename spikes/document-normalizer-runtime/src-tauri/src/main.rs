use tauri_plugin_shell::{ShellExt, process::CommandEvent};

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let sidecar = app.shell().sidecar("cancan-document-agent")?.env_clear();
            let (mut events, mut child) = sidecar.spawn()?;
            tauri::async_runtime::spawn(async move {
                while let Some(event) = events.recv().await {
                    match event {
                        CommandEvent::Stdout(bytes) => {
                            let line = String::from_utf8_lossy(&bytes);
                            if line.contains("\"type\":\"ready\"") {
                                if !line.contains("\"environmentEvidencePresent\":false") {
                                    app_handle.exit(1);
                                    continue;
                                }
                                if child
                                    .write(b"{\"type\":\"ping\",\"requestId\":\"tauri-host\"}\n")
                                    .is_err()
                                {
                                    app_handle.exit(1);
                                }
                            } else if line.contains("\"type\":\"pong\"") {
                                let _ = child.write(b"{\"type\":\"shutdown\"}\n");
                                println!("tauri-sidecar-smoke-passed");
                                app_handle.exit(0);
                            }
                        }
                        CommandEvent::Stderr(bytes) => {
                            eprintln!("sidecar stderr: {}", String::from_utf8_lossy(&bytes));
                        }
                        CommandEvent::Error(error) => {
                            eprintln!("sidecar error: {error}");
                            app_handle.exit(1);
                        }
                        CommandEvent::Terminated(payload) => {
                            eprintln!(
                                "sidecar terminated before host smoke completed: {payload:?}"
                            );
                            app_handle.exit(1);
                        }
                        _ => {}
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("Tauri runtime failed: {error}");
        std::process::exit(1);
    }
}

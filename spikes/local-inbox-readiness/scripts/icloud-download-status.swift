import Darwin
import Foundation

enum CaptureDecision: String, Equatable {
    case ready
    case deferCapture = "defer"
}

enum CaptureReason: String, Equatable {
    case nonUbiquitousRegular = "non-ubiquitous-regular"
    case ubiquitousCurrent = "ubiquitous-current"
    case notRegularFile = "not-regular-file"
    case downloading = "downloading"
    case notDownloaded = "not-downloaded"
    case downloadedMayBeStale = "downloaded-may-be-stale"
    case statusUnknown = "status-unknown"
    case providerError = "provider-error"
}

struct ResourceState {
    let regularFile: Bool?
    let ubiquitousItem: Bool?
    let downloading: Bool?
    let downloadStatus: URLUbiquitousItemDownloadingStatus?
    let uploaded: Bool?
    let uploading: Bool?

    init(
        regularFile: Bool?,
        ubiquitousItem: Bool?,
        downloading: Bool?,
        downloadStatus: URLUbiquitousItemDownloadingStatus?,
        uploaded: Bool? = nil,
        uploading: Bool? = nil
    ) {
        self.regularFile = regularFile
        self.ubiquitousItem = ubiquitousItem
        self.downloading = downloading
        self.downloadStatus = downloadStatus
        self.uploaded = uploaded
        self.uploading = uploading
    }
}

enum PreflightState {
    case resourceValues(ResourceState)
    case providerFailure
}

func captureDecision(for state: PreflightState) -> (CaptureDecision, CaptureReason) {
    guard case let .resourceValues(values) = state else {
        return (.deferCapture, .providerError)
    }
    guard values.regularFile == true else {
        return (.deferCapture, .notRegularFile)
    }
    guard values.ubiquitousItem == true else {
        return (.ready, .nonUbiquitousRegular)
    }
    if values.downloading == true {
        return (.deferCapture, .downloading)
    }
    guard let status = values.downloadStatus else {
        return (.deferCapture, .statusUnknown)
    }

    switch status {
    case .current:
        return (.ready, .ubiquitousCurrent)
    case .notDownloaded:
        return (.deferCapture, .notDownloaded)
    case .downloaded:
        return (.deferCapture, .downloadedMayBeStale)
    default:
        return (.deferCapture, .statusUnknown)
    }
}

func downloadStatusName(_ status: URLUbiquitousItemDownloadingStatus?) -> String {
    guard let status else {
        return "unknown"
    }
    switch status {
    case .notDownloaded:
        return "not-downloaded"
    case .downloaded:
        return "downloaded"
    case .current:
        return "current"
    default:
        return "unknown"
    }
}

func printDecision(for state: PreflightState) {
    let (decision, reason) = captureDecision(for: state)
    if case let .resourceValues(values) = state {
        print("regular-file=\(values.regularFile == true)")
        print("ubiquitous-item=\(values.ubiquitousItem == true)")
        print("uploaded=\(values.uploaded == true)")
        print("uploading=\(values.uploading == true)")
        print("downloading=\(values.downloading == true)")
        print("download-status=\(downloadStatusName(values.downloadStatus))")
    } else {
        print("provider-status=error")
    }
    print("capture-decision=\(decision.rawValue)")
    print("capture-reason=\(reason.rawValue)")
}

func runSelfTest() -> Bool {
    let cases: [(PreflightState, CaptureDecision, CaptureReason)] = [
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: false,
            downloading: false,
            downloadStatus: nil
        )), .ready, .nonUbiquitousRegular),
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: nil,
            downloading: nil,
            downloadStatus: nil
        )), .ready, .nonUbiquitousRegular),
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: true,
            downloading: false,
            downloadStatus: .current
        )), .ready, .ubiquitousCurrent),
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: true,
            downloading: false,
            downloadStatus: .notDownloaded
        )), .deferCapture, .notDownloaded),
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: true,
            downloading: false,
            downloadStatus: .downloaded
        )), .deferCapture, .downloadedMayBeStale),
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: true,
            downloading: true,
            downloadStatus: .current
        )), .deferCapture, .downloading),
        (.resourceValues(ResourceState(
            regularFile: true,
            ubiquitousItem: true,
            downloading: false,
            downloadStatus: nil
        )), .deferCapture, .statusUnknown),
        (.providerFailure, .deferCapture, .providerError),
    ]

    return cases.allSatisfy { state, expectedDecision, expectedReason in
        captureDecision(for: state) == (expectedDecision, expectedReason)
    }
}

guard CommandLine.arguments.count == 2 else {
    fputs("usage: icloud-download-status.swift <candidate-file> | --self-test\n", stderr)
    exit(64)
}

if CommandLine.arguments[1] == "--self-test" {
    guard runSelfTest() else {
        fputs("self-test=failed\n", stderr)
        exit(1)
    }
    print("self-test=passed")
    exit(0)
}

let file = URL(fileURLWithPath: CommandLine.arguments[1])

do {
    let values = try file.resourceValues(forKeys: [
        .isRegularFileKey,
        .isUbiquitousItemKey,
        .ubiquitousItemIsUploadedKey,
        .ubiquitousItemIsUploadingKey,
        .ubiquitousItemIsDownloadingKey,
        .ubiquitousItemDownloadingStatusKey,
    ])
    printDecision(for: .resourceValues(ResourceState(
        regularFile: values.isRegularFile,
        ubiquitousItem: values.isUbiquitousItem,
        downloading: values.ubiquitousItemIsDownloading,
        downloadStatus: values.ubiquitousItemDownloadingStatus,
        uploaded: values.ubiquitousItemIsUploaded,
        uploading: values.ubiquitousItemIsUploading
    )))
} catch {
    printDecision(for: .providerFailure)
}

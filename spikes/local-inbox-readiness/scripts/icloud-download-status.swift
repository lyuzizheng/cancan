import Darwin
import Foundation

guard CommandLine.arguments.count == 2 else {
    fputs("usage: icloud-download-status.swift <synthetic-file>\n", stderr)
    exit(64)
}

let file = URL(fileURLWithPath: CommandLine.arguments[1])

do {
    let values = try file.resourceValues(forKeys: [
        .isUbiquitousItemKey,
        .ubiquitousItemIsUploadedKey,
        .ubiquitousItemIsUploadingKey,
        .ubiquitousItemDownloadingStatusKey,
    ])
    guard values.isUbiquitousItem == true else {
        print("ubiquitous-item=false")
        print("download-status=not-ubiquitous")
        exit(0)
    }

    print("ubiquitous-item=true")
    print("uploaded=\(values.ubiquitousItemIsUploaded == true)")
    print("uploading=\(values.ubiquitousItemIsUploading == true)")
    guard let status = values.ubiquitousItemDownloadingStatus else {
        print("download-status=unknown")
        exit(0)
    }

    switch status {
    case .notDownloaded:
        print("download-status=not-downloaded")
    case .downloaded:
        print("download-status=downloaded")
    case .current:
        print("download-status=current")
    default:
        print("download-status=unknown")
    }
} catch {
    fputs("icloud-download-status: \(error)\n", stderr)
    exit(1)
}

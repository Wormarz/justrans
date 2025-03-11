# JusTrans

A simple file exchange program for LAN networks. JusTrans allows you to easily transfer files between devices using a web browser.

## Features

- Web-based file transfer (no installation needed on the receiving end)
- Simple and intuitive GUI built with Slint
- QR code generation for easy connection
- Drag and drop file uploads
- Works on local networks without internet connection

## Usage

1. Start the JusTrans application
2. The app will display your local IP address and a QR code
3. On another device, open a web browser and navigate to the displayed URL
4. Upload or download files through the web interface

## Building from Source

```
cargo build --release
```

The executable will be available in `target/release/justrans`

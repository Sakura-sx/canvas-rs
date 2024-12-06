# Modern Canvas

A real-time collaborative pixel canvas with WebSocket support. Built with a modern, glass-morphic UI design.

## Features

- 1024x1024 pixel canvas
- Real-time pixel updates via WebSocket
- Modern glass-morphic UI with animated gradient background
- Responsive design for mobile devices
- Live connection status indicator
- Toast notification system

## API

The canvas provides both REST endpoints and WebSocket connections for interaction. See the [documentation](docs) for detailed API specifications.

## Prerequisites

- Rust toolchain (preferrably stable)

## Getting Started

Run the following after opening a shell in the directory:
```sh
RUSTFLAGS="-C target-cpu=native" cargo run --release
```

Run the site
Just visit the site and start placing pixels! No authentication required.

## Tech Stack

- Frontend: Vanilla JavaScript, CSS3
- Backend: Rust w/ Websockets & HTTP
- Canvas: HTML5 Canvas API

# Architecture Overview

TFSV is organized around a single binary and a small set of focused modules.

- `filesystem`: browsing, file metadata, sorting, safety checks
- `transfer`: transfer manifests and SFTP instructions
- `opener`: detection and launching of external file handlers
- `editor`: terminal editor integration placeholder
- `config`: persistent settings and theme configuration
- `ui`: layout and rendering logic
- `events`: keyboard and terminal event handling
- `app`: shared state and main loop

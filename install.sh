#!/bin/sh
set -eu

APP_NAME=tfsv
REPO_DIR=$(cd -- "$(dirname -- "$0")" && pwd -P)
USER_ID=$(id -u)
if [ "$USER_ID" -eq 0 ] && [ -z "${TFSV_INSTALL_DIR+x}" ]; then
    INSTALL_DIR=/usr/local/bin
else
    INSTALL_DIR=${TFSV_INSTALL_DIR:-"$HOME/.local/bin"}
fi
SHARED_BIN_DIR=${TFSV_SHARED_BIN_DIR:-/usr/local/bin}
SHELL_NAME=$(basename -- "${SHELL:-sh}")

case "$SHELL_NAME" in
    bash)
        SHELL_RC=${TFSV_SHELL_RC:-"$HOME/.bashrc"}
        ;;
    zsh)
        SHELL_RC=${TFSV_SHELL_RC:-"$HOME/.zshrc"}
        ;;
    *)
        SHELL_RC=${TFSV_SHELL_RC:-"$HOME/.profile"}
        ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "Rust/Cargo was not found. Installing the stable Rust toolchain..."
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- https://sh.rustup.rs | sh -s -- -y --profile minimal
    else
        printf '%s\n' "Rust installation requires curl or wget. Install one, then rerun ./install.sh." >&2
        exit 1
    fi

    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    else
        PATH="$HOME/.cargo/bin:$PATH"
        export PATH
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        printf '%s\n' "Rust installation completed, but Cargo is not available in this shell." >&2
        exit 1
    fi
fi

if command -v rustup >/dev/null 2>&1; then
    ACTIVE_TOOLCHAIN=$(rustup show active-toolchain 2>/dev/null || true)
    if [ -z "$ACTIVE_TOOLCHAIN" ] && [ -z "${RUSTUP_TOOLCHAIN:-}" ]; then
        printf '%s\n' "No default Rust toolchain is selected. Installing stable..."
        rustup default stable
    fi
fi

printf '%s\n' "Building $APP_NAME in $REPO_DIR..."
if ! (cd "$REPO_DIR" && cargo build --release --bin "$APP_NAME"); then
    printf '%s\n' "Build failed. Review the Cargo output above; your existing installation was not changed." >&2
    exit 1
fi

mkdir -p "$INSTALL_DIR"
INSTALL_DIR=$(cd "$INSTALL_DIR" && pwd -P)
install -m 0755 "$REPO_DIR/target/release/$APP_NAME" "$INSTALL_DIR/$APP_NAME"

if [ "$USER_ID" -eq 0 ]; then
    chmod a+rx "$INSTALL_DIR"
    mkdir -p "$SHARED_BIN_DIR"
    chmod a+rx "$SHARED_BIN_DIR"
    SHARED_BIN_DIR=$(cd "$SHARED_BIN_DIR" && pwd -P)
    if [ "$INSTALL_DIR" != "$SHARED_BIN_DIR" ]; then
        ln -sfn "$INSTALL_DIR/$APP_NAME" "$SHARED_BIN_DIR/$APP_NAME"
    fi
    COMMAND_PATH="$SHARED_BIN_DIR/$APP_NAME"
else
    PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
    if ! grep -Fqx "$PATH_LINE" "$SHELL_RC" 2>/dev/null; then
        mkdir -p "$(dirname -- "$SHELL_RC")"
        {
            printf '\n%s\n' '# TFSV user-local command'
            printf '%s\n' "$PATH_LINE"
        } >> "$SHELL_RC"
    fi

    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *) PATH="$INSTALL_DIR:$PATH" ;;
    esac
    export PATH
    COMMAND_PATH="$INSTALL_DIR/$APP_NAME"
fi

printf '\n%s\n' "TFSV is installed at $INSTALL_DIR/$APP_NAME."
printf '%s\n' "This shell can launch it now with: $INSTALL_DIR/$APP_NAME"
if [ "$USER_ID" -eq 0 ]; then
    printf '%s\n' "All users can run: $COMMAND_PATH"
else
    printf '%s\n' "Open a new terminal (or run: . \"$SHELL_RC\") to use: tfsv"
fi
printf '%s\n' "Run 'tfsv' from any directory to browse."

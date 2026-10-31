# Start the Orus OS session on the first console login.
[ -f "$HOME/.profile" ] && . "$HOME/.profile"
if [ "$(tty)" = /dev/tty1 ] && [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${SSH_CONNECTION:-}" ]; then
    exec orus-session
fi

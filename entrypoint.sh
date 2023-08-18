#!/usr/bin/env bash

while getopts "i" OPT; do
    case "$OPT" in
        i) export QP_SERVER_INSPECTOR=1 ;;
    esac
done

service dbus start
cp -rf /etc/X11/fluxbox ~/.fluxbox
Xvfb $DISPLAY -screen 0 800x600x24 -nolisten tcp &
fluxbox -display $DISPLAY -screen 0 >/dev/null &
x11vnc -nopw -ncache -forever -display $DISPLAY -rfbport $X11VNC_PORT -q &
sleep 3s
echo "xvfb, x11vnc, fluxbox 已成功启动。VNC 远程连接端口: $X11VNC_PORT"

QP_SERVER_PORT=80 /app/qplugged-rust-server

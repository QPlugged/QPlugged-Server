FROM ubuntu:22.04

RUN apt-get update && apt-get install -y wget libgbm-dev libasound-dev dbus xvfb x11vnc fluxbox
RUN wget -q -O /tmp/qq.deb https://dldir1.qq.com/qqfile/qq/QQNT/ad5b5393/linuxqq_3.1.2-13107_amd64.deb &&\
    apt-get install -y /tmp/qq.deb &&\
    rm -f /tmp/qq.deb
RUN mkdir -p /var/opt/qq $HOME/.config && ln -s /var/opt/qq $HOME/.config/QQ

COPY target/release/qplugged-rust-server /app/qplugged-rust-server
COPY target/release/silk-codec /app/silk-codec
COPY entrypoint.sh /app/entrypoint.sh

EXPOSE 80
EXPOSE 5900
ENV DEBIAN_FRONTEND nonintractive
ENV X11VNC_PORT 5900
ENV DISPLAY :99
ENTRYPOINT [ "/app/entrypoint.sh" ]
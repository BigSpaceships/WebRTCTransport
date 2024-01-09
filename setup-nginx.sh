#!/bin/bash
if [ "$1" == "-i" ]; then
    cp ./nginx.list /etc/apt/sources.list.d/nginx.list
    sudo apt update
    sudo apt install -y nginx
fi
rm /etc/nginx/nginx.conf
cp ./nginx.conf /etc/nginx/nginx.conf
sudo systemctl start nginx

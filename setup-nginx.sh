#!/bin/bash
# Run this script manually: bash ~/2dcad/setup-nginx.sh
# It will ask for your sudo password.

sudo ln -sf /home/jit/2dcad/2dcad.dnaboy.org.nginx /etc/nginx/sites-enabled/2dcad.dnaboy.org
sudo nginx -t && sudo systemctl reload nginx
echo "Done! 2dcad.dnaboy.org is now configured on port 8081."

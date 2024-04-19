#!/bin/bash
FILE=gen_netdevs
if [ -f "$FILE" ]; then
    echo "$FILE exists."
else 
    echo "$FILE does not exist. Compiling."
    gcc /home/safeposix-rust/gen_netdevs.c -o gen_netdevs
fi

echo "Generating netdevs"

./gen_netdevs > net_devices
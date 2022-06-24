#!/bin/bash
FILE=gen_netdevs
if [ -f "$FILE" ]; then
    echo "$FILE exists."
else 
    echo "$FILE does not exist. Compiling."
    gcc /home/lind/lind_project/src/safeposix-rust/gen_netdevs.c -o gen_netdevs
fi

echo "Generating netdevs"

./gen_netdevs > net_devices
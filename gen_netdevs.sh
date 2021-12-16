#!/bin/bash
netstat -rn | awk '{print $1}' | tail -n +3 > net_devices

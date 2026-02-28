#!/bin/bash
# Setup script for Simple Quake mouse input
# The game reads raw mouse input from /dev/input/mice to ensure
# mouse look works even while WASD keys are held (a known Linux/winit issue).

echo "Checking /dev/input/mice permissions..."

if [ ! -e /dev/input/mice ]; then
    echo "ERROR: /dev/input/mice does not exist on this system."
    echo "This is unusual — most Linux systems have this device."
    exit 1
fi

if [ -r /dev/input/mice ]; then
    echo "OK: /dev/input/mice is already readable."
else
    echo "/dev/input/mice is NOT readable by current user."
    echo "Running: sudo chmod a+r /dev/input/mice"
    sudo chmod a+r /dev/input/mice
    if [ -r /dev/input/mice ]; then
        echo "OK: Fixed! /dev/input/mice is now readable."
    else
        echo "ERROR: Still not readable. Try running as root."
        exit 1
    fi
fi

echo ""
echo "You can now run the game with: cargo run"


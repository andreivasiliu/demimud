#!/bin/sh

set -e

if ! git --version > /dev/null; then
    echo "Error: git executable not found."
    exit 1
fi

if [ ! -e "license.txt" ]; then
    echo "Error: license.txt not found."
    exit 1
fi

if [ -e "area" || -e "socials.txt" ]; then
    echo "Error: ./area/ or ./socials.txt already exits; rename or remove them first."
    exit 1
fi

git clone https://github.com/mudhistoricalsociety/dawnoftime_1.69r
git clone https://github.com/DikuMUDOmnibus/Ultra-Envy

cp -v -r dawnoftime_1.69r/area area
cp -v Ultra-Envy/sys/SOCIALS.TXT socials.txt

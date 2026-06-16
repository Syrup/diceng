#!/usr/bin/env bash
# Usage: prompt-install.sh <name> <check_cmd> <install_cmd> <snippet>

name="$1"
check_cmd="$2"
install_cmd="$3"
snippet="$4"

if eval "$check_cmd" >/dev/null 2>&1; then
    echo "$name found."
    exit 0
fi

echo "$name not found."
echo ""
read -r -p "Install now? [y/N] " ans
case "$ans" in
    [yY]*)
        echo "Installing $name..."
        eval "$install_cmd"
        ;;
    *)
        echo ""
        echo "Install manually:"
        echo "$snippet"
        echo ""
        exit 1
        ;;
esac

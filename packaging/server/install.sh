#!/bin/sh
# Copy package files only. Never create accounts, edit credentials or start services.
set -eu
if [ "$#" -ne 2 ] || [ "$1" != --root ]; then
    echo 'Usage: ./install.sh --root /absolute/staging-root (or / for explicit host installation)' >&2
    exit 2
fi
case "$2" in /*) root=$2 ;; *) echo 'Installation root must be absolute.' >&2; exit 2 ;; esac
package=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
install -d -m 0755 "$root/usr/local/lib/vmanyware" "$root/usr/lib/sysusers.d" "$root/etc/systemd/system"
install -d -m 0755 "$root/etc/vmanyware"
install -m 0755 "$package/bin/vmanyware-server" "$root/usr/local/lib/vmanyware/vmanyware-server"
install -m 0644 "$package/vmanyware.service" "$root/etc/systemd/system/vmanyware.service"
install -m 0644 "$package/vmanyware.sysusers" "$root/usr/lib/sysusers.d/vmanyware.conf"
install -m 0600 "$package/server.env.example" "$root/etc/vmanyware/server.env.example"
install -m 0644 "$package/templates.yaml.example" "$root/etc/vmanyware/templates.yaml.example"
printf '%s\n' 'Package files copied. Existing server.env and database were not modified.'
printf '%s\n' 'Read INSTALL.md before creating the service account, configuring TLS/authentication or starting the service.'

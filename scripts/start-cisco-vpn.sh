#!/bin/sh

# Required environment: VPN_SERVER, VPN_USER
if test -z "$VPN_SERVER"; then
    echo "VPN_SERVER is not set"
    exit 1
fi

if test -z "$VPN_USER"; then
    echo "VPN_USER is not set"
    exit 1
fi

if test -z "$VPN_IFNAME"; then
    INTERFACE=""
else
    n=0
    while test -e /sys/class/net/${VPN_IFNAME}$n; do
        n=$(( n + 1 ))
    done
    INTERFACE="--interface=${VPN_IFNAME}$n"
fi

USER="$1"

if test -z "$USER"; then
    echo "Usage: $(basename $0) USERNAME"
    exit 1
fi

HOME=$(getent passwd $USER | cut -d: -f6)
CISCO_HOME=$HOME/.cisco

if test -z "$VPN_ASKPASS"; then
    VPN_ASKPASS=systemd-ask-password
fi

PASSWORD1="$($VPN_ASKPASS "Password:")"
PASSWORD2="$($VPN_ASKPASS "OTP:")"

mkdir -p $CISCO_HOME

echo -e "$PASSWORD1\n$PASSWORD2" | env CSD_HOSTNAME=$VPN_SERVER openconnect $INTERFACE --script=/etc/vpnc/systemd-networkd-vpnc --csd-wrapper=/usr/local/bin/csd-wrapper.sh --csd-user=$USER --protocol=anyconnect --user=$VPN_USER $VPN_SERVER

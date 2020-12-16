# systemd-networkd-vpnc

This is a glue between OpenConnect and systemd-networkd.
Works by replacing `vpnc-script`.

Installation:

```
cargo install systemd-networkd-vpnc --root /usr/local/
cp units/cisco-vpn.netdev units/cisco-vpn.network /etc/systemd/network
networkctl reload
```

After it's installed, add `--script=/usr/local/bin/systemd-networkd-vpnc` option to your `openconnect` command.
For instance:

```
openconnect --interface=cisco-vpn --script=/usr/local/bin/systemd-networkd-vpnc \
    --csd-wrapper=/usr/local/bin/csd-wrapper.sh --csd-user=myname --protocol=anyconnect \
    --user=corporate.user@company.com vpn.company.com
```

The script generates `/etc/systemd/network/cisco-vpn.network.d/routes.conf` drop-in config file
and reloads config, so systemd-networkd handles VPN network configuration.

After VPN connection, if everything went well, you will see the following status:

```
$ networkctl
IDX LINK      TYPE     OPERATIONAL SETUP      
  1 lo        loopback carrier     unmanaged  
  2 enp4s0    ether    no-carrier  configuring
  4 wlan0     wlan     routable    configured 
  9 cisco-vpn none     routable    configured

4 links listed.
```

## Licence

Licenced under [MIT](http://opensource.org/licenses/MIT). Feel free to use it at your discretion.
This software is free of charge, no warranty issued!
I'm not responsible for any data, hardware or any other losses as a result of using or misusing this software!

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed under MIT, without any
additional terms or conditions.

All contributions are welcome as PRs here!
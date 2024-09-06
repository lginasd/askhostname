# askhostname

askhostname is cross-platform command-line tool for discovering hosts in local network by acquiring their hostnames and local domain names,
with support of both NetBIOS and mDNS protocols.

# Why I should use askhostname

When you need simple CLI tool to be aware of your local network surroundings.\
Printers and Windows machines can provide their name by NetBIOS name service.
Other devices may use zero-configuration networks like Avahi for Linux and BSD, or Apple Bounjur, which use mDNS.

akshostnames, as name implies, just asks hosts for names.
If you need more features, you might want to use [nbtscan](https://github.com/resurrecting-open-source-projects/nbtscan) or nbtstat.exe for NetBIOS
and [avahi](https://avahi.org/) for mDNS, DNS-SD.

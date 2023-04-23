# huawei-modem

![CC0 licensed](https://licensebuttons.net/p/zero/1.0/88x31.png)
[![Crates.io badge](https://img.shields.io/crates/v/huawei-modem.svg)](https://crates.io/crates/huawei-modem)
[![Docs](https://docs.rs/huawei-modem/badge.svg)](https://docs.rs/huawei-modem)
![GitHub stars](https://img.shields.io/github/stars/eeeeeta/huawei-modem.svg?style=social)

The `huawei-modem` library provides a set of utilities for interfacing with USB 3G/HSDPA/UMTS
modems (particularly Huawei models, like the E220 and E3531) that use the Hayes/AT command set.

At present, the library's main consumer is
[sms-irc](https://git.eta.st/eta/sms-irc). In particular, it may be helpful to
look at [modem.rs](https://git.eta.st/eta/sms-irc/src/branch/master/src/modem.rs) inside that project
to get a feel for how to use this library, as well as looking inside the `examples/`
subdirectory to see some simple SMS sending/receiving examples.

## Status

The library can presently send and decode GSM 7-bit and UCS-2 SMS messages, using the SMS PDU
format. Some edge cases aren't well implemented or need more testing, but the library is
broadly usable for most common SMS sending & receiving needs!

It could do with a bit more ergonomics, however. Currently, if you're confused as to how to
use it, have a look at `sms-irc`'s usage above, look at the examples, or file an issue if
you're still stuck!

## Contributing

This project is passively maintained (i.e. not actively developed, but I generally consider it complete for my use-case and only develop when needed).
[PRs and issues on GitHub](https://github.com/eeeeeta/huawei-modem) are more than welcome!

If you're more old-school, feel free to also [email me a patch](https://git-send-email.io/) the old-fashioned way.

## Licensing

Licensed under CC0.

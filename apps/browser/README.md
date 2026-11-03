# Babel Browser

A browser launcher whose profile, extension list and native process call are
written in **Babel**. Chromium supplies actual website rendering and its
existing browser UI. This is not a new HTML/CSS engine.

The default extension set is **uBlock Origin Lite** (Manifest V3), **Vimium**
(Vim navigation) and **Dark Reader** (dark website themes). The first launch
fetches the three CRX packages from the official Chrome Web Store update
endpoint and installs them as unpacked extensions.

On NixOS:

```sh
nix develop .#browser
bash apps/browser/launch.sh https://example.org
```

For subsequent launches the installed extension directories are reused.
To update them, run `python3 apps/browser/install_extensions.py --force`.
`BABEL_BROWSER_DIR` changes the root of the independent browser profile.

This uses **Chromium, not branded Google Chrome**: current Google Chrome
builds ignore `--load-extension`. The installer relies on an undocumented
Web Store CRX endpoint and will fail with a clear error if Google changes it,
rather than silently claiming that an extension was installed.

**Trust warning:** the Babel launcher uses
`--extensions-on-chrome-urls` to broaden extension access. This does
not remove Chrome's manifest/API permissions or grant arbitrary native OS
access. Chromium's process sandbox, same-origin rules and TLS verification
remain enabled. Extension archives are obtained over HTTPS and checked for
unsafe paths and size, but the installer does *not* cryptographically verify
the CRX signature after unpacking. Only install extensions you trust.

The older Babel Playwright browser primitive remains restricted and is
unaffected by these settings.

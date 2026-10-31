# Orus OS distro (Debian) — Backlog

Orus OS on a Debian stable base: the Detir shell and waid apps on the shared `detir-scene`
runtime, labwc as the compositor, packaged as `.deb`s and a live/installer ISO. This repo holds the
session config, packaging and ISO build; the shell code lives in `orus-universal` and the
runtime in `detir`, both pinned under `src/`. `br2-orus/` stays for fixed hardware
(Pi4, appliance) and is not part of this track.

Chronology, reasoning and what was ruled out are in `work-log/`.

## Phase 1: Linux bring-up in a Debian VM

Status: done except two items.
Decision: Debian stable (Trixie) aarch64 VM on the Mac, reached over SSH. See `work-log/2026-10-13.md`.
Goal: `orus-linux` runs on Linux with a waid app inside the shell.
Still missing for Phase 1 completion: no Linux CI for `orus-linux`; wgpu falls back to llvmpipe in the VM.

- [x] Create a Debian stable aarch64 VM (UTM) with `openssh-server` reachable from the Mac.
      Evidence: UTM VM `orus-debian`, Debian 13.7.0 aarch64, `ssh orus`, `/dev/dri/renderD128` present.
- [x] Remove the dead `detir-wkwebview` overlay and `is_webview` badge from `orus-shell`.
- [x] Make the `detir-webview` GTK backend opt-in (`gtk` feature) so Linux builds do not pull GTK/WebKit.
- [x] Build `orus-linux` from `orus-universal` on the VM.
- [x] Record which wgpu adapter the VM gets.
      Evidence: Vulkan is llvmpipe (CPU); EGL/GLES is virgl on the host GPU, but wgpu's `eglCreateContext` failed (0x3009) and it fell back to llvmpipe.
- [x] Add Linux sans families (`Noto Sans`, `DejaVu Sans`, `Liberation Sans`) to the font lookups in `jag-draw`, `jag-text` and `detir-scene`.
- [x] Open a waid app from the shell on Linux.
      Decision: the shell launches the standalone `detir-scene` once per web bundle under `~/.orus/apps/`.
- [ ] Find why wgpu's GLES adapter fails with `eglCreateContext` 0x3009 on virgl, so the VM uses hardware GL instead of llvmpipe.
- [ ] Add a Linux build job to CI for `orus-linux`.

Dropped:
- The Browser app content rendering only its top part under `cage`: the Browser app was removed (Phase 9) and `cage` is gone.
- Rendering built web apps inside the embedded `SceneRenderer`: the shell no longer embeds `detir-scene`; each app is its own process.
- The flaky headless `detir-scene` start under `cage`: `cage` is gone, and `detir-scene` started in every headless labwc run on 2026-10-20.

## Phase 2: Packages and licenses

Status: started.
Goal: signed-off `.deb`s of the shell and runtime that a Debian machine can install, with every license obligation met.
Ordering: after Phase 1.

- [x] Pin the sources as submodules under `src/` and build them with `scripts/build.sh`.
- [x] Add a `detir-shell` .deb: the shell, `orus`, the labwc session, the Chromium wrapper, the theme, System Settings and a Wayland session entry.
- [x] Add a `waid-runtime` .deb with `detir-scene`.
- [x] Ship a Secret Service in the session (`gnome-keyring`, `libpam-gnome-keyring`, `dbus-user-session`) so the release runtime runs JS.
      Evidence: the live ISO's `user` login unlocks the keyring and System Settings renders from the release runtime; `session/pam-configs/orus-keyring`.
- [ ] Decide the auto-login case: with no password at login the keyring cannot unlock; options are an empty-password keyring or a TPM-backed detir vault.
- [x] Add `debian/copyright` and `LICENSE` with per-package lists of the linked Rust crates.
      Evidence: `debian/copyright`, `licenses/`, `scripts/license-notices.sh`; lintian reports no errors.
- [x] Relicense the runtime under BSL 1.1 (`licenses/LICENSE-waid-runtime-BSL-1.1.txt`): production use allowed, Apache-2.0 four years after each release; replaces the EULA and `scripts/check-eula.sh`.
- [ ] Have counsel confirm the BSL parameters (Additional Use Grant, Change Date, Change License).
- [ ] Write the developer and distributor agreement for uses the Additional Use Grant does not cover.
- [ ] Ship the license texts for the linked crates, not only the list (`cargo about`).
- [ ] Add the notice and source offer MPL-2.0 requires for `cooked-waker`, `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `resvg`, `selectors` and `usvg` in `waid-runtime`.
- [ ] Align `detir`'s `Cargo.toml` license fields with a proprietary runtime: the workspace declares `MIT OR Apache-2.0` (13 crates inherit it) and 11 crates declare none.
- [ ] State a license for `DJS`; it has none declared and is not linked into a shipped binary yet.
- [ ] Credit the 7 Lucide-style stroke icons (ISC needs its notice shipped) and find the source of the 30 coloured 128px logos (language logos and `lock`) before shipping them.
- [x] Ship labwc 0.9.8 (built with wlroots 0.19.3) for the thumbnail Alt+Tab switcher; Trixie has only 0.8.3.
      Evidence: `scripts/build-labwc.sh`, `debian/` labwc package.
- [ ] Provide the labwc and wlroots source with every ISO (GPL-2.0 requires an offer); `/usr/share/doc/labwc/README.source` points to the pinned tarballs.
- [ ] Return to Debian's labwc when a release shipping 0.9 or newer reaches stable or backports.
- [ ] Build the apps in the package build without staged files: `debian/rules` needs `nodejs` and `npm` on the builder for `scripts/build-apps.sh`.
- [ ] Build the packages for x86_64 on an x86_64 builder.
- [x] Ship `react-shadcn-suite` as a built bundle in the package, for testing media and forms; `scripts/build-apps.sh` copies a bundle that has a `waid-package.json` as it is.
      Evidence: distro `17c84fa`; the `detir-shell` deb carries it and the session installs it per user. Its sample media needs its terms checked before a public release (`assets/NOTICE.md`).

## Phase 3: ISO and installer

Status: started.
Goal: a bootable live ISO that installs Orus OS to disk with the Orus OS look and no Debian or Calamares branding.
Ordering: after Phase 2.

- [x] Build an aarch64 live-build ISO with the Detir session at the console login.
      Evidence: `iso/`, `scripts/build-iso.sh`; built and booted in UTM (latest `orus-0.1.0-arm64-38.iso`).
- [x] Boot the ISO in UTM and check the login, keyring unlock, launcher and Settings on a clean system.
- [x] Add the Calamares installer to the ISO with the runtime license step, Orus OS branding and an "Install Orus OS" launcher entry on live media.
- [x] Run an install from the ISO onto a blank UTM disk and boot the installed system.
      Evidence: `orus-0.1.0-arm64-3.iso` installed onto a blank 16 GB disk; the installer user logged in, the keyring unlocked, and Install Orus OS was gone.
- [x] Add a graphical login: `orus-login` (greetd and `orus-greeter` in labwc).
      Evidence: booted ISOs show the greeter and the installed system's user logs in through it.
- [x] Quiet, branded boot: hidden GRUB on the live ISO and installed systems, `quiet` kernel options, and `os-release` naming Orus OS.
      Evidence: `iso/config/hooks/normal/0100-orus-grub.hook.binary`, `installer/modules/grubcfg.conf`. The installed-system part is untested.
- [ ] Check the hidden GRUB on an installed system.
- [ ] Brand the console login banner (`/etc/issue`) and the message of the day, which still say Debian.
- [x] Show "Welcome to Orus OS" and "Let's set up your computer." on the installer's welcome page, with no Calamares text and no About button.
      Evidence: `installer/branding/orus`, `installer/orus-brand-welcome`; rendered headless. The subtitle only shows once the requirements pass, so it is unseen.
- [x] Theme the installer in the desktop's dark colors: `stylesheet.qss`, the orus window chrome, a teal welcome card, a QML sidebar with the version in its footer, a checkmark and radio dot.
      Evidence: `installer/branding/orus`; rendered headless on the build VM.
- [ ] Check every installer page after Welcome on a real install: the headless run stops at Welcome without a usable disk.
      Acceptance: the license accept box, language list frame, slideshow background, Cancel button and subtitle look right.
- [ ] Draw the keyboard page's preview in dark colors; Calamares paints it in fixed light grays.
- [ ] Replace the generated wordmark and welcome image with the real logo, and point the support, release-notes and product URLs at real pages.
- [ ] Replace the postinst patch of Calamares' welcome module (`orus-brand-welcome`): a newer Calamares that changes the two texts leaves the stock wording, and an upgrade does not refresh the copy.
- [ ] Decide whether to replace the Calamares pages with a Detir installer UI, once the install flow is stable.
- [ ] Keep users from enabling auto-login in the installer; it would defeat the keyring unlock.
- [x] Ship the Noto fonts so the installer's language list draws every script.
- [x] Stop the live installer window from being lost: it has only a close button and Alt+Tab switches back to it.
- [ ] Ship Wi-Fi and common NIC firmware (`firmware-iwlwifi`, `firmware-realtek`, `firmware-atheros`, `firmware-linux-free`) for real hardware.
- [ ] Pin the Mesa and kernel versions the ISO ships, using `trixie-backports` only where a GPU needs it.
- [ ] Build a headless server edition ISO (`EDITION=server scripts/build-iso.sh`): Debian base with ssh, NetworkManager and the Debian installer; no `detir-shell`, `waid-runtime`, apps or Calamares.
      Evidence: `iso/server/`, `iso/auto/config`. Not built or booted yet.
- [ ] Build a bootable x86_64 live-build ISO with the Detir session as default.

## Phase 4: Session, panel and compositor

Status: started.
Decision: labwc (wlroots) as the session compositor, the shell as its client. See `work-log/2026-10-13.md`.
Goal: the bar, dock and launcher stay above windows, and apps run as ordinary windows beside them.
Phase: labwc session; the `orus-panel` layer-shell surfaces; apps as processes.
Still missing for Phase 4 completion: the dock lists only five fixed apps; the shell still carries unused in-shell window management.
Ordering: after Phase 2.

- [x] Replace `cage` with labwc in the Detir session, with window rules that keep the desktop window at the bottom.
- [x] Run Terminal, Files, Text Edit and Viewer as windows of their own (`orus-linux --app <name>`).
      Evidence: orus-universal `137a70b`.
- [x] Show Chromium and a waid app as ordinary windows beside the shell.
- [x] Draw the bar, dock and launcher as layer-shell surfaces of `orus-linux --panel`; the bar reserves its strip, and the dock hides under a maximized window and returns at the bottom edge.
      Evidence: orus-universal `d3c8156` (`orus-panel`).
- [x] Read the window list over wlr-foreign-toplevel so the bar names the focused app and the dock reacts to maximized windows.
- [x] Open the launcher from the keyboard with Super+Space (`orus-linux --toggle-launcher` over a socket).
      Evidence: orus-universal `3787c17`, `session/rc.xml`. The key press itself is untested; defining keybinds turns labwc's defaults off, so `session/rc.xml` lists Alt+Tab, Alt+Shift+Tab and Alt+F4.
- [ ] Open the launcher with the Super key alone; labwc keybinds need a non-modifier key.
- [ ] Repeat held keys in the launcher search; the panel reads key presses but not repeats.
- [ ] Check the launcher's typing on a real keyboard; the headless test cannot send keys.
- [ ] Slide the dock in and out and honor Reduce motion; it now appears and disappears at once.
- [x] Show every open app in the dock: the five fixed tiles, then one tile per other app with a window open (Text Edit, Viewer, web apps, others); the running dot comes from the window list, and a click raises the app's next window or starts it.
      Evidence: orus-universal `orus-panel/src/dock_model.rs` (5 tests); on the build VM Text Edit, Viewer and a web app got tiles and clicking one raised its window.
- [ ] Pin and unpin apps in the dock; the five fixed tiles cannot be changed.
- [ ] Give `detir-scene` windows an app id (a `DETIR_WINDOW_APP_ID`, like `DETIR_WINDOW_TITLE`); web apps have none, so the dock matches them by window title, and Settings by the title "Settings".
- [x] Add Sleep, Restart, Shut down and Log out in a menu from the right end of the bar.
- [ ] Check Sleep, Restart and Shut down on a local session (real hardware or an installed VM); they run `systemctl` and show nothing if the system refuses.
- [ ] Ask for confirmation before Restart and Shut down when windows hold unsaved work.
- [x] Restyle Alt+Tab as a row of window thumbnails (labwc 0.9.8).
- [ ] Remove the in-shell window management (window manager, switcher overlay, session restore, frames) from the shell.
- [ ] Decide on the `ORUS/compositor` port. It is deferred, not dropped: 16k lines, but it depends on the missing `rune-compositor` and lacks XWayland, foreign-toplevel, screencopy and text-input.
      Acceptance: reopen only if a detir-specific window protocol or detir-rendered window effects are needed.

## Phase 5: Desktop look and first-party apps

Status: started.
Goal: the desktop, login screen, Settings, Files and Viewer match the dark and light concepts.
Phase: concept steps 1 to 4, Appearance, Files, login screen and Settings.
Still missing for Phase 5 completion: Light theme in the greeter, launcher and in-shell apps; real data in Settings; the Files and Viewer follow-ups below.
Ordering: after Phase 4.

- [x] Concept step 1: wallpaper, the concept palette, and right-aligned line-icon window buttons with rounded corners.
- [x] Concept step 2: restyle the bar (K mark, centered date, network, battery, clock) and add the persistent bottom dock with running dots; workspace dots are left out because labwc has none.
- [x] Concept step 3: restyle System Settings (sidebar, Appearance, Network, Power, Privacy, About) with real data where it exists.
- [x] Concept step 4: rebuild Files with tabs, a places and devices sidebar, a path bar with search, grid and list views, thumbnails, item count and free space, dark and light.
      Evidence: orus-universal `c48386b`, tabs above the right panel `d454a15`. Files stays a native app, so the earlier web-app rebuild is dropped.
- [x] Match window titlebars to the concept: 48px high, left-aligned bold title, minimize/maximize/close on a 50px pitch.
- [x] Match the login screen to the login concept (vignette, K mark, clock, avatar, password with eye toggle, Switch user, keyboard pill, network state, power menu).
- [x] Make Appearance work: theme, accent, reduce motion and wallpaper, shared through `~/.orus/appearance.json` and `orus-appearance`.
- [x] Add a Network section to System Settings on NetworkManager.
- [x] Ship sample pictures in `/etc/skel/Pictures` and open Viewer on `~/Pictures`.
      Evidence: orus-universal `532d23e`, distro `17c84fa`; rendered headless. Viewer now browses the home folder like Files.
- [x] Draw the launcher with the dock's app tiles, the theme's colors and a highlight on the focused app.
      Evidence: orus-universal `launcher.rs`; rendered headless in Dark and Light.
- [ ] Draw the Light theme in the places that stay dark: the greeter and the in-shell Terminal, Text Edit and Viewer.
- [ ] Make web apps other than Settings, and Chromium, follow the theme; the runtime has no `prefers-color-scheme: dark`, so `appearance.json` is the only contract.
- [ ] Let users pick their own wallpaper images; the picker offers only the shipped dunes and "Automatic".
- [ ] Apply Reduce motion beyond Settings: the shell's blinking carets and the runtime's CSS animations.
- [ ] Add the Display section (resolution, scale, layout); it says "not available yet".
- [ ] Add the Sound section on PipeWire, with the bar's sound icon.
- [ ] Show real data on the Privacy page: read app permissions from `orus-sandbox` instead of a fixed list.
- [ ] Give Files and Viewer a decision on a sandbox boundary: both browse the whole home folder, and Files shows "/" as System.
- [ ] Open a file from Files in Text Edit or Viewer.
- [ ] Add Recent and Starred to the Files sidebar; nothing records recent or starred files yet.
- [ ] Give Files a colour label for folders (the concept's teal Design folder) and a code emblem for folders like Workspace; neither has a feature behind it.
- [ ] Show a Files tab indicator like the concept's teal dot; the concept does not say what it means.
- [ ] Redraw Files thumbnails when decoding finishes; today it redraws for 3 seconds after the images change.
- [ ] Draw Files, the shell and the login screen text with Inter; the default font looks thinner than the concept, most visibly in Light.
- [ ] Round the bottom corners of window decorations; labwc rounds only the top, the concept's 14px needs client-side rounding or a compositor change.
- [ ] Show an app icon in the titlebar's icon slot for web apps and Files; labwc finds no icon for these windows.
- [ ] Add the Accessibility control to the login screen; there is no accessibility feature behind it.
- [ ] Let the login screen change the keyboard layout and pick a Wi-Fi network; the keyboard pill and network circle only show state.
- [ ] Draw the login screen in the user's theme once appearance can reach it; it is always dark.

## Phase 6: Media on Linux

Status: started.
Goal: waid apps play video and audio and capture the camera and microphone on Linux.
Phase: the GStreamer backend `detir/crates/linux-media` behind `media-gstreamer`, and `waid-runtime`'s GStreamer and PipeWire dependencies.
Still missing for Phase 6 completion: sound output and real camera and microphone are untested (the build VM has neither); events never reach the page.
Ordering: after Phase 2.

- [x] Play `<video>`, `<audio>` and `new Audio()` through GStreamer.
      Evidence: detir `bc89779c`; 13 tests play real Ogg clips; the react-shadcn-suite probe played `sample.mp4` and `sample.mp3` on the build VM.
- [x] Implement `getUserMedia`: camera through V4L2, microphone through PipeWire or PulseAudio, with levels, samples and `captureFrame`.
      Evidence: detir `linux-media` tests; in a waid app with the capture capabilities the page got a stream and a moving camera preview from test sources.
- [ ] Check sound output, the camera and the microphone on a machine that has them; the device-discovery path has only run against an empty list.
- [ ] Choose a camera or microphone by `deviceId` and `facingMode`; capture opens the first device the system reports.
- [ ] Play a microphone stream back through the speakers when a page attaches it to an `<audio>` element.
- [ ] Deliver `ended`, `canplay` and `durationchange` to web media elements: the web runtime never reads the media host's event queue on any platform (detir `docs/bugs.md`).
- [ ] Play IR-mode `video` nodes on Linux; only the web `<video>` path has a GStreamer backend.

## Phase 7: Web content on Linux

Status: not started.
Goal: real web content renders where detir's own engine does not yet reach parity.
Still missing for Phase 7 completion: the `detir-webview` Linux backend is WebKitGTK on X11 only and has never been built.

- [ ] Build the `detir-webview` Linux backend, or replace it with a Wayland-capable one.
      Reopened: `backend/linux.rs:298-299` does not compile against gtk-rs 0.18 (`X11Window::foreign_new_for_display` takes `&X11Display` and returns `X11Window`).
- [ ] Place a web view inside a detir-scene node on Wayland.

## Phase 8: App isolation on Linux

Status: not started.
Goal: apps run with only the file and network access they are granted.
Still missing for Phase 8 completion: `orus-sandbox` has file, network and audit brokers but no OS-level isolation (no seccomp, Landlock or namespaces).

- [ ] Run each app process under Landlock and seccomp.
- [ ] Route every file and network call from an app through the brokers.
- [ ] Add a test that an app cannot open a path it was not granted.

## Phase 9: Apps as installed web apps

Status: started.
Goal: Orus OS's own apps are installed web apps launched by the shell; the shell has no IR apps and no built-in browser.
Phase: the launcher lists apps from `~/.orus/apps`; `orus` installs them; System Settings is the first one.
Still missing for Phase 9 completion: Files, Viewer, Text Edit and Terminal are still native apps; Settings shows little real data.
Ordering: after Phase 4.

- [x] Build System Settings as a React app in the macOS System Settings layout.
- [x] Remove the IR system apps (Browser, Workflow Editor, Sim Viewer, the IR Settings) and the in-shell browser, address bar and IR windows.
- [x] Install web apps with `orus install` and list them in the launcher.
      Evidence: `orus-core::installed_apps`, `crates/orus`; the launcher lists Settings and react-shadcn-suite.
- [x] Start with an empty desktop: no Terminal or Browser at startup, and a saved Terminal is not restored.
- [x] Remove `detir-scene` and V8 from `orus-shell` by embedding the file manager and image viewer icons.
      Evidence: shell release binary 89 MB to 28 MB.
- [ ] Add a scoped host filesystem capability to detir: list, stat, create, rename, move, delete and binary read within the roots the sandbox grants.
      Acceptance: a web app can browse and manage files; today `host.fs` is text-only read and write through picker dialogs.
- [ ] Rebuild Viewer and Text Edit as web apps.
- [ ] Rebuild Terminal as a web app; it needs a PTY host capability.
- [ ] Remove the in-process app threads and the `orus-filemanager`, `orus-imageviewer`, `orus-textedit` and `orus-terminal` crates from the shell once they are web apps.
- [x] Add Bluetooth, Display, Sound and Mouse & Touchpad sections to System Settings.
      Evidence: `apps/system-settings`; `orus-appearance input|display`; 25 Settings tests and 5 core tests. Checked on the VM by screenshot and CLI: Display applies and survives a labwc restart; Mouse & Touchpad writes the labwc `<libinput>` rules; Bluetooth and Sound show their empty states.
- [x] Add a Printers section to System Settings: detect network (mDNS) and USB printers, add them driverless, test page, default, remove.
      Evidence: `apps/system-settings` (`printers.ts`, `Printers.tsx`); `cups`, `cups-browsed`, `avahi-daemon`, `ipp-usb` in `detir-shell`. On the VM the page lists the CUPS PDF printer; finding and adding a real printer is untested.
- [ ] Check Printers against a real network and a USB printer, and that the live-session user is in `lpadmin` (the installed user is, from `users.conf`).
- [ ] Let apps print: a Print action in Text Edit, Files, Viewer and web apps through CUPS.
- [ ] Add scanning: `sane-airscan` plus a scan app, and scanners in Settings.
- [ ] Check Bluetooth (pair, connect, forget), the touchpad and mouse settings, and Sound on real hardware: the VM has no adapter, touchpad or sound device.
- [ ] Pair Bluetooth devices that ask for a PIN or passkey (needs a BlueZ agent in the session).
- [ ] Pick the sound device (default output and input) in Settings; today it sets the volume of the default ones.
- [ ] Add a Keyboard section (layout, repeat rate) and Date & Time (time zone, 24-hour clock) to System Settings.
- [x] Add multi-screen options to Display: turn a screen on or off, rotate it, place it left, right, above or below another.
      Evidence: `orus-appearance display set OUTPUT --pos --transform --on|--off`; checked on the VM with two headless outputs (CLI and page). Not tested on real monitors.
- [ ] Mirror screens (labwc has no mirror mode), pick a main screen, and add night light.
      Acceptance: needs a host capability that exposes each setting to a web app; none exists on Linux today.
- [ ] Give installed web apps an icon from the app's manifest; the launcher and dock show a generic app tile for them (Settings has its own).
- [ ] Install from an archive or a URL, not only a directory.

## Phase 10: App store and updates

Status: started (the `orus` command and the store app work against a development catalog; signing keys, the default source and the apt repository are not built).
Goal: install, update, roll back and cleanly remove apps from signed catalogs, `.orusapp` files and URLs, from the `orus` command and a store app.
Phase: the `orus` crate (`sources`, `catalog`, `archive`, `sign`), a store app, and an apt repository for system packages.
Ordering: after Phase 9.

- [x] Define the catalog (`index.json` and a detached Ed25519 `index.json.sig`), the `.orusapp` zip format and the sources files: `/etc/orus/sources.d/*.toml` (system) and `~/.orus/config/sources.toml` (user).
      Evidence: orus-universal `crates/orus` (`catalog`, `archive`, `sign`, `sources`); 13 tests. The system sources folder and the keys are not shipped in the packages yet.
- [x] Install from a folder, a `.orusapp` file, an https URL or a catalog name, with a permissions screen and a SHA-256 check before anything is unpacked.
      Evidence: `orus install`; tests cover a changed download, a catalog that disagrees with its archive, a path that leaves the folder, and a declined review. The https URL path is untested against a real server.
- [x] Add `orus sources list|add|remove|enable|disable`; `add --development <url>` allows plain `http` only for loopback and private addresses, marks the source unsigned and always shows the permissions screen.
- [x] Add `orus update`, `search` and `upgrade` against the signed catalogs; the default source is `https://packages.kilne.dev`, and no development source ships in the ISO.
- [x] Keep one installed version per app; `orus install app@1.2.0` picks a version and `orus rollback app` swaps back to the single previous version.
- [x] Add `orus catalog build|serve` (default port 6678) and `catalog keygen`, so an app built on the Mac installs on a VM through the development source.
- [x] Remove an app completely: `orus purge` removes the runtime's stored data first (a failed purge leaves the app installed), then the bundle and rollback copy; `orus remove` keeps the data, as in apt.
      Acceptance: cookies and the shared sign-in are shared across apps and are reported as left, not removed. Still to do: stop a running app first.
      Acceptance: a test installs an app, runs it so it stores data and a secret, removes it, and finds nothing left of it in `~/.detir/detir.db`, the keyring or the folders.
- [ ] Declare the files an app creates outside its own folders in `waid-package.json` (`artifacts`, limited to paths the app was granted), show them at install, and remove them only with `orus remove --purge`; user-made documents are never removed by a plain `orus remove`.
- [ ] Put everything the runtime creates for an app under `~/.orus/data/<appId>/` (data, cache, config), so removing the app's data is one delete.
- [ ] Add `orus clean`: find data left by apps that are no longer installed (earlier removals, crashes) and delete it after a confirmation.
- [x] Add a purge to the runtime (`detir-scene purge-app <dir> [--dry-run]`): web storage, pinned entry, vault and installation keyring scopes, app sign-in. Evidence: detir uncommitted; 6 unit tests plus a real-keychain test on the VM.
- [ ] Purge the app's `history`, `pages` rows and caches in `detir-scene purge-app`.
- [x] Build the store app (a web app): browse and search, install with a permissions screen, open, update, roll back, remove (with or without its data), and a Sources screen.
      Evidence: `apps/orus-store`; tested on the VM against a development catalog (install, open and the Installed page by clicking). It runs `orus --json` through `host.shell`; `orus install --allow` refuses an archive that asks for more than the screen showed.
- [ ] Install the exact source the store showed: `orus install` picks the highest version across sources, so two sources offering the same id can install from the one not shown.
- [x] Give the store app an icon in the launcher and dock. Evidence: orus-universal `DockIcon::Store`.
- [x] Rename Image Viewer to Viewer and open PDFs in it: first-page thumbnails in the gallery, one page at a time in the preview (arrows, wheel or a click at either side turn pages; past the ends they move to the next file).
      Evidence: orus-universal `orus-viewer` (`pdf.rs`); 3 tests; checked on the VM by screenshot with `assets/samples/Pictures/Orus-Sample.pdf`.
- [x] Add `openssh-server` to the ISO, off on live media and on for installed systems.
      Evidence: `iso/config/package-lists/orus.list.chroot`, `ssh.service.d/orus-not-live.conf` (`ConditionPathExists=!/run/live/medium`). Untested on a booted ISO.
- [ ] Add an x86 (amd64) build: build and boot the ISO on a real x86 laptop (`ARCH=amd64`); the x86 pdfium library, labwc package and BIOS GRUB path have not run.
- [ ] Follow the VirtualBox window size: the session does not resize when the VM window changes. Debian 13 has no `virtualbox-guest-utils` (the `virtualbox` source left Trixie), and VirtualBox only gives the window size to the guest through Guest Additions, so a `wlr-randr` helper alone gets no event (tested: setvideomodehint changed neither the kernel mode list nor sent a hotplug on a Debian 13 VM). Workaround: pick a mode in Settings > Display. Options: a small client that reads `/dev/vboxguest` and sets the layout on `vmwgfx`, or Oracle's Additions on installed systems.
- [ ] Boot under Secure Boot: the amd64 ISO carries `/EFI/boot/bootx64.efi` and `grubx64.efi` but no `shim` or signed GRUB package, so a machine with Secure Boot on (an HP 245 ships that way) should refuse it until Secure Boot is turned off. Not tested: EFI and Secure Boot have not run, even in a VM. Add Debian's signed `shim-signed` and `grub-efi-amd64-signed` to the image, or document turning Secure Boot off for the first release.
- [x] Undo a display change that is not kept within 15 seconds (a "Keep these display settings?" bar on the Display page).
      Evidence: `apps/system-settings` (`restoreChange`, `Display.tsx`); the countdown reverted a rotation on the VM and saved the old one.
- [x] Lay out an open Detir window again when the output scale changes (it kept the old scale until reopened, so Settings looked half-size at 200%).
      Evidence: detir `detir-scene` runner (`refresh_scale_factor`, `WindowEvent::ScaleFactorChanged`); on the VM an open Settings window re-laid out at 200%. The SMAA setting made at start is not redone for the new scale.
- [x] Replace the native dropdowns in Settings with a themed `Select`.
      Evidence: `apps/system-settings/src/ui.tsx`; the runtime's `<select>` list ignores option colors, so the page's light text showed on white.
- [ ] Draw the native `<select>` list in Detir with the option and page colors, as Chrome does (browser parity); then drop the Settings `Select` if it adds nothing.
- [x] Add a boot menu to the live ISO: Install Orus OS (opens the installer at once, the default after 5 seconds) or Try Orus OS (the login screen; the installer stays in the launcher).
      Evidence: `iso/config/hooks/normal/0100-orus-grub.hook.binary` (`orus.mode=install|live`), `installer/live/live-greetd`, `live-try.toml`; the hook was run on the build VM's real `grub.cfg`. The booted menu is untested.
- [ ] Log in to the live session without typing a password (the keyring needs the password login, so Try asks for the live user's, `live` by default); show a hint on the login screen.
- [x] Keep the Store and Settings out of the Store's Installed list and refuse to remove or purge them.
      Evidence: orus-universal `orus::store::SYSTEM_APPS`; `orus list --json` has `system`; 1 test.
- [x] Add an Office tile to the launcher on installed systems: it starts LibreOffice, or opens a Terminal that runs `sudo apt-get install` for Writer, Calc and Impress when it is missing; LibreOffice windows share one dock tile.
      Evidence: orus-universal `external_app::launch_office`, `ORUS_TERMINAL_COMMAND`; 3 launcher tests; the Terminal typed and ran a given command on the VM. The real install and LibreOffice under Wayland are untested.
- [ ] Open `.docx`, `.xlsx` and `.pptx` files in LibreOffice: needs a default app per file type.
- [ ] Ask for the install in the installer once it has a network step.
- [x] Draw PDFs with pdfium and keep `hayro` as the fallback when the library is missing.
      Evidence: orus-universal `orus-viewer` (`pdfium-render`, library found through `ORUS_PDFIUM`, `/usr/lib/orus/libpdfium.so`, beside the program); `scripts/fetch-pdfium.sh` pins bblanchon/pdfium-binaries `chromium/8111` by SHA-256 for arm64 and x64; `detir-shell` ships it with its licenses. 5 tests pass on hayro and on pdfium (VM, including a password-protected file refused with a clear message); checked by screenshot.
- [ ] Check the x64 build: the x64 library is fetched and hashed but was not run.
- [ ] Ask for the password of a protected PDF in the Viewer (pdfium can open it; the Viewer has no input for it).
- [ ] Move PDF viewing to pdf.js (`pdfjs-dist`, shipped locally) once Detir has Canvas 2D, Workers and a binary file read; then drop `hayro` and `pdfium`.
- [ ] Prune the Viewer's rendered-page cache in `~/.orus/cache/viewer`; it only grows.
- [ ] Add zoom, text selection, search and links to the PDF view; the Viewer shows only the whole page.
- [ ] Check that opening an image or PDF in Files (double-click) opens it in the Viewer.
- [ ] Sign app archives and catalogs with a Orus OS key, with the public key shipped in `/usr/share/orus/keyrings/`.
- [ ] Publish a signed apt repository for `detir-shell`, `waid-runtime`, `labwc` and the installer package, with the key in the ISO.
- [ ] Ask the runtime for a minimum version in the catalog (`min_runtime`) and refuse an app that needs a newer one.
- [ ] Decide whether to keep Debian Chromium, whose profile menu still shows "Google services settings", or ship ungoogled-chromium (not in Debian; needs our own package). Optionally hide Guest and Add profile with `BrowserGuestModeEnabled` and `BrowserAddPersonEnabled` in `session/orus-policy.json`.
- [x] Add Users & Groups to System Settings: lists accounts with their groups and admin flag; add, change password, admin on/off and delete each open a Terminal that runs `sudo`.
      Evidence: `apps/system-settings` (`users.ts`, `Users.tsx`, 4 tests); the add/admin/remove commands ran on the build VM; the Terminal opens from outside the shell on the live ISO. The page itself is not checked by screenshot.
- [ ] Check Users & Groups on the live ISO and an installed system, including that the list refreshes after the Terminal command ends.
- [x] List programs installed with apt (GIMP, LibreOffice apps) in the launcher, from their `.desktop` files; `session/share/applications` hides the Debian ones that do not belong (Calamares, KDE Settings, Print Settings, Zutty).
      Evidence: `orus-core/src/desktop_apps.rs` (3 tests). Not yet run in a session.
- [ ] Draw the real icon of a launcher program from the icon theme; `.desktop` programs get the generic app tile.

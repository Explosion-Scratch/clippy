<div align=center><a href="https://explosion-scratch.github.io/clippy"><img width=128 src="./src-tauri/icons/icon.png"/></a></div>
<div align=center><h1>Clippy</h1></div>

A tiny MacOS clipboard manager built with Tauri and Rust. Clippy features interactive search, cycling through clipboard items, and a Mac native style interface. Clippy supports images, files, text and html clipboard types, and stores all items simply though the file system. This means items can be saved, restored, manipulated, analyzed, or merged with old saves simply by copy pasting items from folders. The full application comes in at 30MB. Clippy also comes bundled with a fully featured CLI called [get_clipboard](./get_clipboard/README.md) and a beautiful dashboard.

## Screenshots

<table>
  <tr>
    <td align="center" width="25%">
      <img src="frontend/public/images/app/color.png" alt="Small and minimal" width="100%" />
      <br />
      <b>Small and minimal</b>
    </td>
    <td align="center" width="25%">
      <img src="frontend/public/images/app/files.png" alt="Lots of formats supported" width="100%" />
      <br />
      <b>Lots of formats supported</b>
    </td>
    <td align="center" width="25%">
      <img src="frontend/public/images/app/settings.png" alt="Configure wherever" width="100%" />
      <br />
      <b>Configure wherever</b>
    </td>
    <td align="center" width="25%">
      <img src="frontend/public/images/app/tray.png" alt="Manage from the Tray" width="100%" />
      <br />
      <b>Manage from the Tray</b>
    </td>
  </tr>
</table>

## Architecture

The application frontend for Clippy is simply a wrapper that calls the API, hosted by the core `get_clipboard` binary. `get_clipboard` creates and runs a service listening for new clipboard items, while the application shows, copies and injects these items on demand. As `get_clipboard` operates through a persistent LaunchAgent (respecting start / stop / unload commands), it can always be listening for new items in the background without the Cilppy app being open. The clipboard manager UI simply searches, and displays these items from the backend.

# Settings & Configuration UI

To reduce the need for users to manually edit raw configuration files, `julesctl` will introduce a dedicated **Settings & Configuration UI** overlay.

## Features & Capabilities

*   This pane will provide a visual interface to manage the global state stored in the `~/.config/julesctl/` directory (See: [Data Management](../architecture/data-management-ahenk.md)).
*   Users will be able to:
    *   Configure project-agnostic AI rules.
    *   Update credentials securely (utilizing the OS `keyring`).
    *   Adjust UI preferences.
    *   Configure **Default Editors (Vim vs VSCode)** for conflict resolution (See: [Conflict Resolution Framework](../conflict-resolution/framework.md)).
    *   Set default **Prompt Destinations**.
    *   Monitor **Ahenk Sync Statuses** directly from the TUI.

## Design

*   The interface will follow the responsive 2-pane principles established in [Layout and Navigation](layout-and-navigation.md), ensuring accessibility on both desktop and mobile/Termux environments.
*   Like the chat interface, it acts as an overlay/modal, preventing clutter on the main workflow dashboard.

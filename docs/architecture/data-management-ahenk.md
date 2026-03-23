# Data Management & Cross-Device Sync (Ahenk)

`julesctl` aims to provide a seamless cross-device experience (e.g., transitioning from a Desktop PC to an Android phone running Termux) without cluttering project repositories or relying on centralized cloud storage for personal application state.

## 1. Zero Repository Bloat (No `.julesctl` Folder)

*   We strictly avoid creating proprietary configuration folders (like `.julesctl`) inside user repositories.
*   Project-specific AI rules should rely on universally accepted meta-prompting files like `AGENTS.md` or `.gsd/context.md`.
*   **Reason:** Application state, workflow relations, and UI preferences are personal to the developer. Committing them to a shared Git repository causes unnecessary bloat and history pollution.

## 2. Global State Storage (`~/.config/julesctl`)

*   All persistent application state must be stored in the OS-level global configuration directory (`~/.config/julesctl/` on Linux/Mac/Termux).
*   This includes:
    *   Active Workflow/Tab definitions.
    *   The hierarchical mapping of Jules sessions (Session A spawned Session B).
    *   API activity cache (JSON message history downloaded from Jules).
*   Credentials (API Keys) are securely stored in the OS `keyring` and never saved in plaintext files.

## 3. Cross-Device Synchronization via Ahenk (P2P)

*   `julesctl` will integrate the [Ahenk](https://github.com/Appaholics/Ahenk) library for Peer-to-Peer (P2P) network synchronization between devices (e.g., PC <-> Phone).
*   **Scope of Ahenk Sync:** Ahenk is exclusively used to synchronize the personal state stored in `~/.config/julesctl`. When you open `julesctl` on your phone, Ahenk pulls your active tabs, workflow structures, and cached Jules chat history from your PC.
*   **Benefits:** This prevents the phone client from having to re-fetch identical chat histories from the Jules API, saving network bandwidth and quota limits, while providing an instant, seamless UX transition.

## 4. Code Synchronization (Strictly Git-First)

*   Ahenk will **never** be used to synchronize source code, local unpushed commits, or resolve merge conflicts between devices.
*   Both the PC and the Phone (Termux) are treated as "Thick Clients" with their own cloned Git repositories.
*   **Rule:** If a user wants to view or patch AI code on their phone, they must fetch it directly from the remote origin (`git fetch`) via the standard Git protocol. We do not reinvent code synchronization.

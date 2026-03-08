# 49modman

49modman is a desktop mod manager for modding Lethal Company v49. Its focus is to enable support on this legacy version while giving the tools for weeding out incompatible mods.

## Download and Install

### Windows 10/11 (MSI)
1. Open the public releases page: `https://github.com/TheRealCrast/49modman/releases`
2. Download the latest `.msi` file.
3. Run the installer.

To update, run a newer `.msi` build. MSI upgrade detection is enabled and will update an existing install.

### Linux (AppImage)
1. Open the public releases page: `https://github.com/TheRealCrast/49modman/releases`
2. Download the latest `.AppImage` file.
3. Download the installer script:

```bash
curl -fsSL https://raw.githubusercontent.com/TheRealCrast/49modman/main/scripts/install-appimage.sh -o install-49modman.sh
chmod +x install-49modman.sh
./install-49modman.sh ./49modman.AppImage
```

To update, run the same installer script again with a newer `.AppImage`. The existing local install is replaced.

In newer updates, there will be a one-liner that'll make installing this application much quicker on Linux.

## Build From Source

Prerequisites:
- Node.js + npm
- Rust toolchain
- Tauri system dependencies for your platform

```bash
npm install
npm run tauri:dev
```

Production build:

```bash
npm run tauri:build
```

Platform-specific release bundles:

```bash
npm run release:build:linux # Linux
npm run release:build:windows # Windows
```

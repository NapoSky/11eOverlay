# Cross-compilation Windows depuis Linux

Ce projet cible `x86_64-pc-windows-msvc` (PE/COFF natif Windows)  
en utilisant **cargo-xwin** comme runner cargo, qui orchestre :

- `xwin` pour télécharger le Windows SDK (CRT headers + MSVC libs)
- `clang-cl` comme compilateur croisé
- `lld-link` comme linker

Aucun Wine, VM, ni installation Visual Studio n'est requis.

---

## Pré-requis système

### Dépendances apt (Debian 12 / Ubuntu 22.04+)

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  pkg-config \
  clang \
  llvm \
  libdbus-1-dev \
  libglib2.0-dev \
  libsoup-3.0-dev \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libjavascriptcoregtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libssl-dev \
  libxdo-dev
```

> Les paquets `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, etc. ne sont pas utilisés lors
> de la cross-compilation Windows elle-même, mais sont nécessaires pour que
> `cargo check` et `tauri dev` fonctionnent en mode Linux.

### Vérification rapide

```bash
clang --version        # Debian clang version 14+
llvm-config --version  # idem
rustc --version        # 1.80+
node --version         # 20+
```

---

## Installation de la toolchain Rust Windows

```bash
# Ajouter la cible MSVC 64-bit
rustup target add x86_64-pc-windows-msvc
```

Vérification :

```bash
rustup target list --installed | grep windows
# x86_64-pc-windows-msvc  ✓
```

---

## Installation de cargo-xwin

```bash
cargo install cargo-xwin
```

> cargo-xwin télécharge automatiquement `xwin` si absent.  
> Vérification : `cargo xwin --version`

---

## Premier build (téléchargement du Windows SDK)

```bash
pnpm run package
```

Ce script exécute :

```
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
```

### Ce qui se passe au premier lancement

1. **xwin** télécharge depuis les CDN Microsoft :
   - Windows CRT headers (`ucrt`)
   - MSVC runtime libraries (`vc`)
   - SDK headers + libs
2. Tout est mis en cache dans `~/.xwin-cache/` (~1 Go)
3. `cargo-xwin` injecte les chemins dans les variables d'environnement (`INCLUDE`, `LIB`, `LIBPATH`)
4. Cargo compile avec clang-cl comme compilateur et lld-link comme linker

Les builds suivants n'effectuent aucun téléchargement réseau supplémentaire.

---

## Sortie du build

```
src-tauri/target/x86_64-pc-windows-msvc/release/11eOverlay.exe
```

Le flag `--no-bundle` évite la génération d'un installeur MSI/NSIS
(ce qui nécessiterait des outils Windows supplémentaires).

---

## Dépannage

### `error: linker 'lld-link' not found`

```bash
sudo apt install lld
```

### `clang-cl: command not found`

`clang-cl` est un lien symbolique vers `clang`. S'il est absent :

```bash
sudo apt install clang
which clang-cl || sudo ln -s $(which clang) /usr/local/bin/clang-cl
```

### `xwin` échoue à télécharger le SDK

Problème réseau ou proxy. Forcer via :

```bash
XWIN_ACCEPT_LICENSE=1 cargo xwin build --target x86_64-pc-windows-msvc
```

Ou pré-télécharger manuellement :

```bash
cargo install xwin
xwin --accept-license splat --output ~/.xwin-cache/splat
```

### Erreur de certificat SSL lors du téléchargement xwin

```bash
# Mettre à jour les certificats CA
sudo apt install --reinstall ca-certificates
update-ca-certificates
```

### Le binaire Windows ne démarre pas (DLL manquante)

Le build utilise `rustls` (pas d'OpenSSL natif) et les crates en pur Rust.
Le seul prérequis runtime côté Windows est **WebView2** (pré-installé sur Windows 10/11).

Pour distribuer sur une machine sans WebView2 :
- Utiliser `tauri build` avec le bundler NSIS (nécessite un environnement Windows)
- Ou inclure le redistributable WebView2 dans le déploiement

---

## Variables d'environnement utiles

| Variable | Utilisation |
|---|---|
| `XWIN_ACCEPT_LICENSE=1` | Accepter la licence MS SDK sans prompt interactif |
| `XWIN_CACHE_DIR` | Changer le répertoire de cache (défaut : `~/.xwin-cache`) |
| `CARGO_TARGET_DIR` | Changer le répertoire de sortie Cargo |
| `RUST_LOG=debug` | Logs détaillés Tauri/Rust en dev |

---

## Configuration `.cargo/config.toml` (optionnel)

cargo-xwin configure automatiquement le linker au runtime. Si tu constates
des erreurs de linker répétées, tu peux forcer la configuration manuellement :

```toml
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
linker = "clang"
rustflags = ["-C", "linker-flavor=msvc", "-C", "link-arg=-fuse-ld=lld"]
```

> Dans la pratique, ce fichier n'est **pas nécessaire** avec cargo-xwin ; il gère
> ces flags lui-même.

---

## Résumé des commandes

```bash
# Setup unique (à faire une fois)
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
sudo apt install clang llvm lld  # si pas déjà présents

# Build Windows depuis Linux
pnpm run package
# → src-tauri/target/x86_64-pc-windows-msvc/release/11eOverlay.exe
```

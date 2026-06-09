# Release Flow — 11eOverlay

Le workflow `release.yml` se déclenche sur tout tag `v*.*.*`, quelle que soit la branche source.  
Il construit `11eOverlay.exe` sur `windows-latest` et publie une release GitHub contenant un zip avec **uniquement** l'exécutable.

---

## Cas 1 — Release depuis `main`

Flux standard. `main` est stable et prêt à shipper.

```
main
 │
 ├── commits...
 │
 └── [tag v1.2.0] ──────────────────► workflow release.yml
                                             │
                                             ├── checkout@v4 (tag v1.2.0)
                                             ├── pnpm ci
                                             ├── pnpm test
                                             ├── pnpm run build:vite
                                             ├── tauri build --no-bundle
                                             ├── Compress-Archive → 11eOverlay-windows-x86_64.zip
                                             └── softprops/action-gh-release → Release "v1.2.0"
```

### Étapes

```bash
# 1. S'assurer d'être sur main, à jour
git checkout main
git pull origin main

# 2. Mettre à jour la version dans les manifestes
#    - package.json         → "version": "1.2.0"
#    - src-tauri/Cargo.toml → version = "1.2.0"
#    - src-tauri/tauri.conf.json → "version": "1.2.0"

# 3. Commit de version
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: bump version to 1.2.0"
git push origin main

# 4. Tagger et pousser le tag
git tag v1.2.0
git push origin v1.2.0
```

Le workflow démarre automatiquement dès la réception du tag.

---

### Ré-écraser un tag existant (depuis main)

Si le build a échoué, qu'un fichier a été oublié dans le commit de version ou que la release GitHub doit être remplacée.

```bash
# 1. Corriger ce qui doit l'être sur main, puis commit + push
git add ...
git commit -m "fix: ..."
git push origin main

# 2. Supprimer le tag localement et sur le remote
git tag -d v1.2.0
git push origin --delete v1.2.0

# 3. Supprimer la release GitHub associée (si elle a été publiée)
gh release delete v1.2.0 --yes

# 4. Re-poser le tag sur le nouveau HEAD de main
git tag v1.2.0
git push origin v1.2.0
# → le workflow redémarre proprement sur le bon commit
```

> **Note :** `git push --force` sur un tag n'est pas recommandé et ignoré par certains
> clients. La séquence delete + re-create est plus fiable et explicite.

---

## Cas 2 — Release depuis une branche de release

Pour préparer une version sans merger dans `main` au préalable (hotfix, pre-release, RC).

```
main
 │
 ├── commits...
 │
 └─── release/1.3.0 (branche)
       │
       ├── commits de fix/préparation...
       │
       └── [tag v1.3.0-rc.1] ──────────► workflow release.yml
                                                │
                                                ├── checkout@v4 (tag v1.3.0-rc.1)
                                                ├── pnpm ci / test / build...
                                                └── Release "v1.3.0-rc.1" (pre-release)
```

### Étapes

```bash
# 1. Créer la branche de release depuis main (ou depuis le commit cible)
git checkout main
git pull origin main
git checkout -b release/1.3.0

# 2. Effectuer les ajustements nécessaires (changelogs, bumps, fixes)
#    - package.json         → "version": "1.3.0"
#    - src-tauri/Cargo.toml → version = "1.3.0"
#    - src-tauri/tauri.conf.json → "version": "1.3.0"

git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: bump version to 1.3.0"
git push origin release/1.3.0

# 3. Tagger depuis cette branche
git tag v1.3.0-rc.1
git push origin v1.3.0-rc.1
# → workflow déclenché, release publiée en pre-release

# 4. Une fois validé, tagger la release finale
git tag v1.3.0
git push origin v1.3.0
# → release stable publiée

# 5. Merger dans main et supprimer la branche
git checkout main
git merge --no-ff release/1.3.0
git push origin main
git branch -d release/1.3.0
git push origin --delete release/1.3.0
```

---

## Convention de tags

| Tag | Usage |
|---|---|
| `v1.2.0` | Release stable |
| `v1.3.0-rc.1` | Release candidate (pre-release) |
| `v1.3.0-beta.1` | Beta publique (pre-release) |
| `v1.3.0-test` | Test du pipeline, pas une vraie release |

`softprops/action-gh-release` détecte automatiquement les suffixes `-rc`, `-beta`, `-alpha` et marque la release GitHub comme **pre-release**.

---

## Vérifier une release

```bash
# Lister les releases publiées
gh release list

# Télécharger et vérifier le contenu du zip
gh release download v1.2.0 --pattern "*.zip"
unzip -l 11eOverlay-windows-x86_64.zip
# Archive:  11eOverlay-windows-x86_64.zip
#   Length      Date    Time    Name
# ---------  ---------- -----   ----
#  ...        2026-xx-xx xx:xx   11eOverlay.exe
# → un seul fichier attendu
```

---

## Supprimer un tag publié par erreur

```bash
# Supprimer localement et sur le remote
git tag -d v1.2.0
git push origin --delete v1.2.0

# Supprimer la release GitHub associée (si elle a été créée)
gh release delete v1.2.0 --yes
```

---

## Fichiers à versionner à chaque release

| Fichier | Champ |
|---|---|
| `package.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` (section `[package]`) |
| `src-tauri/tauri.conf.json` | `"version"` |

Les trois doivent être synchronisés avant de poser le tag.

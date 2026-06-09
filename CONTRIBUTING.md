# Contribuer à 11eOverlay

Merci de l'intérêt ! Voici comment vous pouvez aider.

---

## Mettre à jour le lexique

Le mode **Lexique** de l'overlay est alimenté par le fichier [`data/content.json`](data/content.json).

Il est organisé en sections :

| Section | Contenu |
|---|---|
| `Armes` | Sigles d'armes (AT, ATR, FA…) |
| `Véhicules` | Abréviations de véhicules |
| `Structures` | Fortifications, bunkers, etc. |
| `Facilities` | Structures de production |
| `Logistique` | Termes logistiques (logi, supply run…) |
| `Termes Généraux` | Abréviations radio, tactiques, etc. |

### Format d'une entrée

```json
{
  "key": "AT",
  "value": "Antichar"
}
```

- `key` : le sigle ou abréviation tel qu'utilisé en jeu
- `value` : la définition en clair

### Proposer un ajout ou une correction

1. Forkez le dépôt.
2. Modifiez [`data/content.json`](data/content.json) :
   - Pour **ajouter** un terme : ajoutez un objet `{ "key": "...", "value": "..." }` dans la section appropriée, en respectant l'ordre alphabétique des `key`.
   - Pour **corriger** une définition : modifiez uniquement le champ `value`.
   - Pour **créer une nouvelle section** : ajoutez un objet `{ "section": "...", "items": [...] }` à la racine du tableau.
3. Ouvrez une Pull Request avec une description courte de la modification.

---

## Signaler un bug

Ouvrez une issue sur [GitHub](https://github.com/NapoSky/11eOverlay/issues) en décrivant :
- Ce que vous avez fait
- Ce que vous attendiez
- Ce qui s'est passé à la place
- Votre version de l'overlay (visible dans la barre de titre)

---

## Contribuer au code

Consultez la structure du projet dans le [README](README.md) avant de commencer.  
Lancez les tests avant de soumettre une PR :

```bash
pnpm test
pnpm run typecheck
```

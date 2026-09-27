# Script Midair

Script Midair 1X1 100% accurate

## Crédits

https://github.com/PhaKouille

Original midair calculator concept and scripts by PhaKouille

## Lancer

```
compile_midair.bat
```

Compile et produit `dist\ScriptMidair.exe`, un exécutable autonome. Il faut
[Rust](https://rustup.rs), aucune autre dépendance.

```
cargo run --release      # lancer directement
cargo build --release    # -> target\release\ScriptMidair.exe
cargo test               # tests de non-régression
```

Sans interface :

```
ScriptMidair.exe --cli preset.json          # sortie texte
ScriptMidair.exe --cli preset.json --json   # sortie JSON
```

## Tri des résultats

Les résultats peuvent être triés selon plusieurs critères, cumulables : chaque
critère coché ajoute son rang, et les résultats sont classés par somme des
rangs.

| Critère            | Effet                                                                     |
| ------------------- | ------------------------------------------------------------------------- |
| Moins de dispensers | Total power sand + power hammer + ratio le plus faible                    |
| Moins d'écart      | Écart moyen entre les trois quantité le plus faible                     |
| Écart Sand/Hammer  | Écart entre power sand et power hammer le plus faible                 |
| Distance            | Distance parcourue par le sand, au choix la plus grande ou la plus petite |

## Paramètres

| Champ                     | Rôle                                                         |
| ------------------------- | ------------------------------------------------------------- |
| Power Sand / Power Hammer | Position du power sand/hammer dans le barrel                  |
| Sand / Hammer             | Position du sand / hammer dans le barrel                      |
| Limit Power               | Quantité limite de power pour les ratios trouvés            |
| GT Hammer Max             | Gametick max du hammer                                        |
| Diff Gametick             | Différence de gametick entre les 2 power (power sand/hammer) |
| Limit Hammer              | Quantité limite de hammer pour les ratios trouvés           |
| Hauteur Adj               | Hauteur à laquelle votre adj est en Y                        |
| Ratio 1x1                 | Ratio 1X1 ou non 1X1 (à cocher)                              |
| Axis                      | Axe X ou Z selon où vous tirez                               |

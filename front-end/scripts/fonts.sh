#!/bin/sh
# Gera as fontes do site só com o alfabeto latino e os ícones de src/lib/icons.ts.
# Rode de novo depois de adicionar um ícone. Precisa do uv.
set -e
cd "$(dirname "$0")/.."
icons=$(grep -o '\\u[0-9a-f]\{4\}' src/lib/icons.ts | sed 's/\\u/U+/' | sort -u | paste -sd,)
for weight in Medium Bold; do
  uvx --from 'fonttools[woff]' pyftsubset "assets/fonts/JetBrainsMonoNerdFont-$weight.ttf" \
    --unicodes="U+20-7E,U+A0-17F,U+2010-2026,U+20AC,U+2190-2193,$icons" \
    --layout-features= --flavor=woff2 --output-file="src/fonts/JetBrainsMono-$weight.woff2"
done

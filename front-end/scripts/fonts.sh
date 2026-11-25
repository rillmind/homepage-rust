#!/bin/sh
# Gera as fontes do site só com o alfabeto latino. Precisa do uv.
set -e
cd "$(dirname "$0")/.."
for weight in Medium Bold; do
  uvx --from 'fonttools[woff]' pyftsubset "assets/fonts/JetBrainsMonoNerdFont-$weight.ttf" \
    --unicodes="U+20-7E,U+A0-17F,U+2010-2026,U+20AC,U+2190-2193" \
    --layout-features= --flavor=woff2 --output-file="src/fonts/JetBrainsMono-$weight.woff2"
done

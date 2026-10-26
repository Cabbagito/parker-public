#!/usr/bin/env bash
set -euo pipefail

kinds=(quote haiku ascii-diagram ascii-drawing table mermaid fact mini-essay)
kind="${kinds[RANDOM % ${#kinds[@]}]}"

personal=(
  "Blues guitar craft and theory"
  "Tube amplifier electronics"
  "AI alignment research"
  "Mechanistic interpretability and sparse autoencoders"
  "Rationality and clear thinking"
  "Effective altruism and longtermism"
  "Software engineering craft"
  "Hard sci-fi (Asimov, Banks, Adams)"
  "Post-scarcity futures"
  "Anime and comics as narrative form"
  "Survival horror game design"
  "Hypertrophy and strength training science"
  "Skateboarding"
  "Basketball"
  "Waterpolo"
  "Chess"
  "Culinary chemistry"
  "Meditation and contemplative practice"
  "Botany and houseplants"
  "Space exploration and astronautics"
  "Architecture and built spaces"
)

wildcard=(
  "Ancient history" "Medieval history" "20th century history" "Economic history" "Military history"
  "Number theory" "Topology" "Statistics" "Game theory" "Information theory"
  "Quantum mechanics" "Relativity" "Thermodynamics" "Optics" "Acoustics"
  "Chemistry" "Geology" "Climatology" "Volcanology"
  "Astronomy" "Cosmology"
  "Linguistics" "Etymology"
  "Music theory" "Cartography" "Mythology"
  "Philosophy of mind" "Ethics" "Logic"
  "Cryptography" "Code-breaking"
  "Evolution" "Ecology" "Mycology" "Ornithology" "Entomology"
  "Neuroscience" "Genetics" "Paleontology"
  "Materials science" "Metallurgy"
  "Architecture history" "Industrial design" "Urban planning"
  "Transportation history" "Aviation history" "Robotics history"
  "Espionage"
  "Art history" "Theater" "Cinema technique" "Typography"
  "Board games" "Sports history" "Weather phenomena"
)

roll=$((RANDOM % 100))
if (( roll < 30 )); then
  topic="${personal[RANDOM % ${#personal[@]}]}"
elif (( roll < 90 )); then
  topic="${wildcard[RANDOM % ${#wildcard[@]}]}"
else
  a="${personal[RANDOM % ${#personal[@]}]}"
  b="${wildcard[RANDOM % ${#wildcard[@]}]}"
  topic="${a} AND ${b} (find a real, non-trivial connection)"
fi

web_roll=$((RANDOM % 100))
if (( web_roll < 10 )); then
  websearch="Enabled"
  allowed_tools_arg=(--allowed-tools WebSearch)
else
  websearch="Disabled"
  allowed_tools_arg=()
fi

cols="${COLUMNS:-100}"
rows=$(( ${LINES:-30} * 8 / 10 ))

prompt="Kind: ${kind}
Topic: ${topic}
WebSearch: ${websearch}
Canvas: at most ${cols} columns wide and ${rows} rows tall (less is fine).

Rules:
- Output ONLY the markdown. No preamble, no explanation.
- Start with a short bold title line.
- The canvas is a MAXIMUM, not a target — if the piece is naturally shorter, leave it shorter.
- Use fenced code blocks for ASCII art, diagrams, or tables that need monospace alignment.
- Be concrete and specific. Pick a particular angle on the topic, not a generic overview.
- Avoid these overused 'delightful fact' tropes unless the topic genuinely demands one:
  octopuses, mantis shrimp, bioluminescence, slime mold, mycelial networks, tardigrades,
  the Voyager Golden Record, the Antikythera mechanism.
- If WebSearch is Enabled: you may search if and only if doing so surfaces something
  genuinely surprising that connects to the topic. Otherwise ignore the web.
- If the topic contains 'AND': find a real, non-trivial connection between the two."

claude -p --model opus --output-format text "${allowed_tools_arg[@]}" "$prompt" | glow -w "$cols" -

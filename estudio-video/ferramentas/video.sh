#!/usr/bin/env bash
# Atalhos de ffmpeg do estúdio de vídeo. Uso: ferramentas/video.sh ajuda
set -euo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
X264=(-c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -movflags +faststart)
AAC=(-c:a aac -b:a 320k -ar 48000)

falha() { echo "erro: $*" >&2; exit 1; }
precisa() { [[ $# -ge 1 && -n "${1:-}" ]] || falha "argumentos faltando; veja: video.sh ajuda"; }

ajuda() {
  cat <<'EOF'
video.sh <comando> [argumentos]

  novo <cliente> <projeto>                  cria a pasta do projeto com os modelos
  verificar                                 confere ferramentas instaladas
  info <arquivo...>                         resolução, fps, codec, duração e áudio
  proxy <entrada> [saida]                   cópia leve 540p para revisar
  trecho <entrada> <ini> <fim> <saida>      recorte preciso (hh:mm:ss.ms)
  vertical <entrada> <saida> [desfoque|corte]  16:9 para 9:16 em 1080x1920
  juntar <lista.txt> <saida>                concatena (formato do concat demuxer)
  sobrepor <base> <grafico> <saida> [ini_s] sobrepõe gráfico com alfa a partir de ini_s
  legendar <entrada> <legenda.ass|srt> <saida>  queima a legenda
  medir <entrada>                           LUFS integrado e pico real
  loudness <entrada> <saida> [alvo=-14]     normaliza em duas passagens
  exportar <entrada> <vertical|youtube|feed|web> <saida>  perfil por plataforma
  quadros <entrada> <pasta> [intervalo_s=2] folha de contato para revisão
  grade <entrada> <pasta>                   quadros com a área segura do vertical
EOF
}

verificar() {
  for f in ffmpeg ffprobe python3 node npx; do
    if command -v "$f" >/dev/null; then echo "ok    $f"; else echo "falta $f"; fi
  done
  python3 -c "import faster_whisper" 2>/dev/null && echo "ok    faster-whisper" \
    || echo "falta faster-whisper (pip install faster-whisper) para transcrição"
  local filtros; filtros="$(ffmpeg -hide_banner -filters 2>/dev/null || true)"
  grep -qw ass <<<"$filtros" && echo "ok    filtro ass (libass)" \
    || echo "falta filtro ass: ffmpeg sem libass"
}

novo() {
  precisa "${1:-}"; precisa "${2:-}"
  local p="$RAIZ/projetos/$1/$2"
  mkdir -p "$p"/{brutos,ia,audio,graficos,render,entregas}
  for m in briefing roteiro plano-edicao qa; do
    [[ -f "$p/$m.md" ]] || cp "$RAIZ/modelos/$m.md" "$p/$m.md"
  done
  # marca é do cliente: compartilhada entre projetos
  [[ -f "$RAIZ/projetos/$1/marca.md" ]] || cp "$RAIZ/modelos/guia-marca.md" "$RAIZ/projetos/$1/marca.md"
  # sem link simbólico (Windows): aponta para o guia do cliente
  [[ -f "$p/marca.md" ]] || printf '# Marca\n\nGuia do cliente em [../marca.md](../marca.md).\n' > "$p/marca.md"
  echo "projeto criado: $p"
}

info() {
  precisa "${1:-}"
  for f in "$@"; do
    echo "== $f"
    ffprobe -v error -show_entries \
      stream=codec_type,codec_name,width,height,r_frame_rate,pix_fmt,sample_rate,channels:format=duration,bit_rate \
      -of default=noprint_wrappers=1 "$f"
  done
}

proxy() {
  precisa "${1:-}"
  local s="${2:-${1%.*}_proxy.mp4}"
  ffmpeg -hide_banner -y -i "$1" -vf "scale=-2:540" -c:v libx264 -preset veryfast -crf 28 \
    -pix_fmt yuv420p -c:a aac -b:a 96k "$s"
}

trecho() {
  precisa "${4:-}"
  ffmpeg -hide_banner -y -ss "$2" -to "$3" -i "$1" "${X264[@]}" "${AAC[@]}" "$4"
}

vertical() {
  precisa "${2:-}"
  local modo="${3:-desfoque}" vf
  case "$modo" in
    desfoque)
      vf="[0:v]split=2[a][b];[a]scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,boxblur=25:2,eq=brightness=-0.08[bg];[b]scale=1080:-2[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,setsar=1[v]" ;;
    corte)
      vf="[0:v]crop=ih*9/16:ih,scale=1080:1920,setsar=1[v]" ;;
    *) falha "modo deve ser desfoque ou corte" ;;
  esac
  ffmpeg -hide_banner -y -i "$1" -filter_complex "$vf" -map "[v]" -map 0:a? "${X264[@]}" "${AAC[@]}" "$2"
}

juntar() {
  precisa "${2:-}"
  ffmpeg -hide_banner -y -f concat -safe 0 -i "$1" "${X264[@]}" "${AAC[@]}" "$2"
}

sobrepor() {
  precisa "${3:-}"
  local ini="${4:-0}"
  ffmpeg -hide_banner -y -i "$1" -itsoffset "$ini" -i "$2" \
    -filter_complex "[0:v][1:v]overlay=(W-w)/2:(H-h)/2:eof_action=pass[v]" \
    -map "[v]" -map 0:a? "${X264[@]}" "${AAC[@]}" "$3"
}

legendar() {
  precisa "${3:-}"
  local leg; leg="$(realpath "$2")"
  leg="${leg//\\/\\\\}"; leg="${leg//:/\\:}"; leg="${leg//\'/\\\'}"
  local filtro="subtitles"; [[ "$2" == *.ass ]] && filtro="ass"
  ffmpeg -hide_banner -y -i "$1" -vf "$filtro='$leg'" "${X264[@]}" -c:a copy "$3"
}

medir() {
  precisa "${1:-}"
  ffmpeg -hide_banner -nostats -i "$1" -af ebur128=peak=true -f null - 2>&1 \
    | sed -n '/Summary:/,$p' | grep -E 'I:|Peak:|LRA:'
}

loudness() {
  precisa "${2:-}"
  local alvo="${3:-"-14"}" json
  json="$(ffmpeg -hide_banner -nostats -i "$1" -af "loudnorm=I=$alvo:TP=-1:LRA=11:print_format=json" -f null - 2>&1 \
    | sed -n '/^{/,/^}/p')"
  local mi mtp mlra mth off
  read -r mi mtp mlra mth off < <(python3 -I -c '
import json,sys; d=json.loads(sys.stdin.read())
print(d["input_i"],d["input_tp"],d["input_lra"],d["input_thresh"],d["target_offset"])' <<<"$json")
  ffmpeg -hide_banner -y -i "$1" -af \
    "loudnorm=I=$alvo:TP=-1:LRA=11:measured_I=$mi:measured_TP=$mtp:measured_LRA=$mlra:measured_thresh=$mth:offset=$off:linear=true,aresample=48000" \
    -c:v copy "${AAC[@]}" "$2"
}

exportar() {
  precisa "${3:-}"
  local vf br
  case "$2" in
    vertical) vf="scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2"; br=16M ;;
    feed)     vf="scale=1080:1350:force_original_aspect_ratio=increase,crop=1080:1350"; br=12M ;;
    youtube)  vf="scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2"; br=18M ;;
    web)      vf="scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2"; br=5M ;;
    *) falha "perfil deve ser vertical, feed, youtube ou web" ;;
  esac
  # fps constante, igual ao da entrada (ffmpeg 5.1+)
  ffmpeg -hide_banner -y -i "$1" -vf "$vf,setsar=1" -fps_mode cfr -c:v libx264 -profile:v high \
    -preset slow -crf 18 -maxrate "$br" -bufsize "$br" -pix_fmt yuv420p -movflags +faststart \
    "${AAC[@]}" "$3"
}

quadros() {
  precisa "${2:-}"
  local int="${3:-2}"; mkdir -p "$2"
  ffmpeg -hide_banner -loglevel error -y -i "$1" -vf "fps=1/$int,scale=270:-2,tile=6x4" -frames:v 1 -update 1 "$2/folha.png"
  echo "folha de contato: $2/folha.png"
}

grade() {
  precisa "${2:-}"
  mkdir -p "$2"
  # área segura do vertical: x 60-940, y 220-1500 (skill video-formatos)
  ffmpeg -hide_banner -loglevel error -y -i "$1" -vf "fps=1/2,scale=1080:1920,\
drawbox=x=0:y=0:w=1080:h=220:color=red@0.35:t=fill,\
drawbox=x=0:y=1500:w=1080:h=420:color=red@0.35:t=fill,\
drawbox=x=940:y=220:w=140:h=1280:color=red@0.35:t=fill,\
drawbox=x=0:y=220:w=60:h=1280:color=red@0.35:t=fill,scale=360:-2" "$2/grade_%03d.png"
  echo "quadros com área segura em $2/"
}

cmd="${1:-ajuda}"; shift || true
case "$cmd" in
  ajuda|-h|--help) ajuda ;;
  novo|verificar|info|proxy|trecho|vertical|juntar|sobrepor|legendar|medir|loudness|exportar|quadros|grade) "$cmd" "$@" ;;
  *) ajuda; exit 1 ;;
esac

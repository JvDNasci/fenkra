#!/usr/bin/env bash
# Produção reproduzível do vídeo "CAD paramétrico em 10 s". Rode de dentro desta pasta.
set -euo pipefail
K=../../..; V=$K/ferramentas/video.sh; SFX=$K/biblioteca/sfx
[[ -f $K/biblioteca/fontes/catalogo.md ]] || python3 -I $K/ferramentas/baixar_fontes.py
[[ -f $SFX/catalogo.csv ]] || python3 -I $K/ferramentas/gerar_sfx.py
mkdir -p render entregas audio
python3 -I $K/ferramentas/trilha.py --clima tecnologico --duracao 10 --saida audio/trilha.wav
python3 -I graficos/render_cad.py render/cad_video.mp4
# trilha + efeitos sonoros nos tempos de sfx_tempos.txt (tempo arquivo ganho)
args=(-i audio/trilha.wav); filt="[0:a]volume=0.55[t];"; lab="[t]"; k=1
while read -r t f g; do
  [[ -z "$t" || "$t" == \#* ]] && continue
  args+=(-i "$SFX/$f"); ms=$(python3 -c "print(int($t*1000))")
  filt+="[$k:a]volume=$g,adelay=$ms|$ms[s$k];"; lab+="[s$k]"; k=$((k+1))
done < sfx_tempos.txt
filt+="${lab}amix=inputs=$k:normalize=0:duration=first,atrim=0:10,afade=t=out:st=9.6:d=0.4[a]"
ffmpeg -v error -y "${args[@]}" -filter_complex "$filt" -map "[a]" -c:a pcm_s16le audio/mix.wav
$V cor render/cad_video.mp4 render/cad_cor.mp4 tech-limpo 0.5
ffmpeg -v error -y -i render/cad_cor.mp4 -i audio/mix.wav -map 0:v -map 1:a -c:v copy -c:a aac -b:a 320k -shortest render/cad_som.mp4
$V loudness render/cad_som.mp4 render/cad_norm.mp4 -14
$V exportar render/cad_norm.mp4 vertical entregas/fenkra-cad-parametrico-10s.mp4
$V quadros entregas/fenkra-cad-parametrico-10s.mp4 render/qa 0.5
$V grade entregas/fenkra-cad-parametrico-10s.mp4 render/qa
echo "pronto: entregas/fenkra-cad-parametrico-10s.mp4"

// SPDX-License-Identifier: LGPL-2.1-or-later
// Leitor web do pacote rodando fora do navegador (B181): carrega do próprio visualizador.html as
// funções que abrem o ZIP e o GLB (com o decodificador meshopt) e resume cada .fkv em JSON. O teste
// em Rust compara o resumo com o do leitor de referência: os dois leitores têm de ver o mesmo pacote.
// Uso: node leitor_web.mjs <visualizador.html> <a.fkv> [b.fkv ...]
import { readFileSync } from "node:fs";
import vm from "node:vm";

const [html, ...arquivos] = process.argv.slice(2);
const fonte = readFileSync(html, "utf8");
const ini = fonte.indexOf("/* ---------------- zip e glb ---------------- */");
const fim = fonte.indexOf("/* ---------------- estado do pacote");
if (ini < 0 || fim < 0) throw new Error("trecho do leitor não encontrado na página");
const ctx = { TextDecoder, Blob, Response, DecompressionStream, DataView, Uint8Array, T: (k) => k, console };
vm.createContext(ctx);
vm.runInContext(fonte.slice(ini, fim) + "\nthis.lerZip = lerZip; this.lerGLB = lerGLB;", ctx);

const tipos = { 5126: Float32Array, 5120: Int8Array, 5123: Uint16Array, 5125: Uint32Array };
const comps = { SCALAR: 1, VEC3: 3 };
const saida = [];
for (const a of arquivos) {
  const b = readFileSync(a);
  const buf = b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength);
  const zip = await ctx.lerZip(buf);
  const m = JSON.parse(new TextDecoder().decode(await zip.ler("pacote.json")));
  const r = { arquivo: a, formato: m.formato, versao: m.versao, tipo: m.tipo, nome: m.nome, arvore: m.arvore.map((e) => e.id), malhas: m.malhas.length,
    passos: (m.passos || []).map((e) => e.passos.length), baloes: (m.baloes || []).map((x) => x.item), folhas: m.folhas.length, compactado: false, acessores: [] };
  if (m.cena) {
    const g = ctx.lerGLB(await zip.ler(m.cena));
    r.compactado = !!g.compactado;
    for (const ac of g.json.accessors) {
      const v = g.json.bufferViews[ac.bufferView], T_ = tipos[ac.componentType], nc = comps[ac.type];
      const passo = v.byteStride ? v.byteStride / T_.BYTES_PER_ELEMENT : nc;
      const arr = new T_(g.buf, g.bin + (v.byteOffset || 0) + (ac.byteOffset || 0), Math.max(0, (ac.count - 1) * passo + nc));
      let soma = 0;
      for (let i = 0; i < ac.count; i++) for (let k = 0; k < nc; k++) soma += arr[i * passo + k];
      r.acessores.push(soma);
    }
  }
  // medição como na página: distância entre pontos de captura de cada malha
  // (centros de círculo, tipo 2, todos contra todos), em precisão dupla e sem kernel
  r.medidas = m.malhas.map((x) => {
    const c = ((x.capturas && x.capturas.pontos) || []).filter((q) => q[3] === 2), d = [];
    for (let i = 0; i < c.length; i++) for (let j = i + 1; j < c.length; j++) d.push(Math.hypot(c[i][0] - c[j][0], c[i][1] - c[j][1], c[i][2] - c[j][2]));
    return d;
  });
  saida.push(r);
}
process.stdout.write(JSON.stringify(saida));

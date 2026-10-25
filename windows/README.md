# Crunchy AI — Windows x64

Nome do aplicativo Windows: **Crunchy AI** (slogan: **Som crocante**). Nome
proposto para publicação: **Crunchy AI — Windows Port** (repositório sugerido:
`Crunchy-AI-Windows-Port`). Este é um fork/port comunitário independente do
[Fovty/HushMic](https://github.com/Fovty/HushMic), não uma versão oficial do
projeto upstream. Preservar os avisos e licenças MIT/Apache-2.0 aplicáveis.

Windows 11 x64 é o alvo prioritário. Windows 10 x64 é alvo secundário e precisa
de validação própria.

## Arquitetura escolhida

O Crunchy AI usa o VB-CABLE oficial instalado separadamente pelo usuário. O app
captura o microfone físico por WASAPI/CPAL, reamostra para 48 kHz, processa o
áudio com o DPDFNet original via ONNX Runtime e envia o resultado ao endpoint
de reprodução `CABLE Input (VB-Audio Virtual Cable)`. Discord/OBS devem usar
`CABLE Output (VB-Audio Virtual Cable)` como microfone. O app não instala,
remove nem altera o driver VB-CABLE.

Pipeline: microfone físico → captura WASAPI/CPAL → fila limitada → DPDFNet
Rust/ONNX → saída WASAPI para CABLE Input → captura VB-CABLE em Discord/OBS.
O aplicativo não grava a captura em disco nem transmite pela rede. Ao parar, o
stream de reprodução fecha e a entrada virtual fica em silêncio.

## Compilar e testar

Rust stable `x86_64-pc-windows-msvc` e Visual Studio 2022 C++ Build Tools são
necessários. Uma instalação local de Rust em `.build-tools/cargo` também é usada
automaticamente pelos scripts.

```powershell
./windows/build-app.ps1
./windows/package-installer.ps1
./dist/CrunchyAI-Setup-x64.exe --verify
./dist/HushMic-Windows-x64/HushMic.exe --devices
./dist/HushMic-Windows-x64/HushMic.exe --check-vbcable
./dist/HushMic-Windows-x64/HushMic.exe --self-test
```

`package-installer.ps1` gera `dist/CrunchyAI-Setup-x64.exe`, um instalador único
x64 que instala o app no perfil do usuário, cria o atalho Crunchy AI e registra o
desinstalador em Apps instalados. Não pede elevação e não instala nem remove
VB-CABLE. Também gera `dist/CrunchyAI-Windows-x64-App.zip` como pacote portátil;
esse ZIP pode ser extraído e instalado com `install-local.ps1`.

`build-app.ps1` baixa os modelos DPDFNet e ONNX Runtime de fontes fixadas e
verifica seus SHA-256. Mantenha `assets` junto do executável. `--self-test` usa
áudio sintético e não exige VB-CABLE. `--devices` lista os microfones físicos.
`--check-vbcable` confirma se CABLE Input e CABLE Output estão enumerados.

O instalador apresenta o app como Crunchy AI e usa o logo fornecido na janela,
no ícone do executável, na barra de tarefas e no atalho do Menu Iniciar. A paleta
usa carvão/grafite com acentos âmbar e coral inspirados no logo; os controles e
fluxos de áudio permanecem os mesmos.

Para executar o port completo, instale o VB-CABLE separadamente pelo instalador
oficial da [VB-Audio](https://vb-audio.com/Cable/), reinicie se o instalador
solicitar, confirme que o Windows mostra os endpoints CABLE Input e CABLE Output
e clique em **Verificar driver** no Crunchy AI. No Discord/OBS, escolha `CABLE
Output (VB-Audio Virtual Cable)` como dispositivo de entrada. O Crunchy AI envia o
processamento a `CABLE Input`. Nenhum driver é incluído, instalado, removido ou
alterado pelos scripts deste projeto; o VB-CABLE deve ser removido pelo
desinstalador próprio da VB-Audio.

O aplicativo instala e desinstala somente o Crunchy AI no perfil do usuário. Use
Apps instalados do Windows para removê-lo; essa ação preserva o VB-CABLE.

## Estado e validação

O app x64 compila, os testes automatizados e autotestes dos modelos passam. Em
uma sessão real de Windows 11 com VB-CABLE instalado separadamente, os endpoints
foram detectados e o usuário confirmou que o retorno processado funcionou com
boa supressão. Ainda faltam medições controladas de estabilidade e latência ponta
a ponta; Windows 10 também não foi validado.

O port não contém nem instala driver HushMic próprio. Secure Boot permanece
ativo; não habilite test signing para usar o VB-CABLE oficial.

O instalador incorpora e valida o ZIP do app antes de ser gerado; `--verify`
confere os arquivos essenciais antes da instalação. As preferências de microfone,
modelo, modo e intensidade são salvas em `%APPDATA%\HushMic\preferences.ini`;
atualizar a lista preserva o microfone selecionado quando ele continua conectado.
O backend C otimizado para Linux não é usado; esta versão executa ONNX na CPU.
## Licenças

HushMic: MIT ou Apache-2.0. Modelos DPDFNet: Apache-2.0. ONNX Runtime: MIT e
avisos de terceiros acompanham os assets. VB-CABLE é software da VB-Audio e
precisa ser obtido/instalado separadamente; consulte `THIRD_PARTY_NOTICES.md` e
o site oficial.

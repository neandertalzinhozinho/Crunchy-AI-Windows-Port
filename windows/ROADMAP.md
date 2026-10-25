# Plano de entrega — Crunchy AI para Windows x64

## Objetivo

Port independente do HushMic para Windows 11 x64, usando VB-CABLE instalado separadamente pelo usuário como dispositivo de microfone virtual. O app não instala, remove nem altera drivers. O port não inclui driver HushMic próprio.

## Estado

- Implementados: interface Windows, captura WASAPI/CPAL, reamostragem para 48 kHz, inferência local dos dois modelos DPDFNet, saída para CABLE Input, monitoramento local opcional e preferências persistentes.
- Implementados: instalador por usuário, desinstalação do app, pacote portátil, identidade visual Crunchy AI e CI Windows que testa e publica artefatos de compilação.
- Verificados: testes do denoiser e app, Clippy, compilação x64, autotestes de áudio sintético e validação do payload de instalação.
- Teste manual em Windows 11 com VB-CABLE instalado separadamente confirmou o retorno processado e boa supressão. Ainda não é um benchmark controlado.
- Ainda pendentes: sessão prolongada com medidas de latência ponta a ponta e contadores; validação sistemática com Discord/OBS, clientes simultâneos, desconexão/reconexão, suspensão/retomada e remoção da versão final; validação no Windows 10.

## Próximas etapas

1. Medir uma sessão longa no Windows 11, capturando o dispositivo virtual em Discord ou OBS e registrando latência, underruns e descartes.
2. Investigar e corrigir crescimento de contadores, caso ocorra em teste controlado.
3. Cobrir formatos de entrada 44,1/48/96 kHz, troca/reconexão de microfone, mute/bypass, monitoramento, clientes simultâneos e suspensão/retomada.
4. Verificar instalação e remoção da versão Crunchy AI em perfil limpo e garantir que VB-CABLE permanece intacto.
5. Validar Windows 10 x64 se continuar dentro do escopo.
6. Publicar o repositório independente com atribuição explícita ao Fovty/HushMic.

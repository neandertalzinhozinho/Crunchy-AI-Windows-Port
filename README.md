<p align="center">
  <img src="windows/assets/crunchy-ai-icon.png" alt="Crunchy AI" width="180">
</p>

<h1 align="center">Crunchy AI — Som crocante</h1>

<p align="center">Redução local de ruído para microfone no Windows 11 x64.</p>

> **Port/fork independente do [Fovty/HushMic](https://github.com/Fovty/HushMic).** Não é uma versão oficial do projeto original. Créditos e licenças MIT/Apache-2.0 originais permanecem nos arquivos deste repositório.

## Instalação

O instalador será disponibilizado como artefato CI após a primeira publicação e execução do workflow Windows.

1. Instale o [VB-CABLE oficial da VB-Audio](https://vb-audio.com/Cable/) separadamente e reinicie se o instalador solicitar.
2. Baixe e execute o artefato `CrunchyAI-Windows-x64` na execução mais recente do workflow Windows em Actions. O instalador do Crunchy AI é por usuário e não requer elevação.
3. Abra o Crunchy AI, escolha seu microfone físico e inicie o processamento.
4. No Discord, OBS ou outro programa, selecione **CABLE Output (VB-Audio Virtual Cable)** como microfone.

O VB-CABLE **não está incluído** no instalador do Crunchy AI. O app não instala, remove ou altera o driver. Para desinstalar, remova Crunchy AI em **Aplicativos instalados**; remova VB-CABLE pelo desinstalador da VB-Audio.

O processamento e a captura do microfone permanecem locais. O modo de monitoramento opcional toca o áudio processado na saída padrão — use fones para evitar microfonia. O projeto não grava sua captura em disco nem envia áudio pela rede.

## Desenvolvimento

Veja o [guia Windows](windows/README.md), o [estado de validação](windows/VALIDATION.md) e o [plano de entrega](windows/ROADMAP.md). Windows 11 x64 é o alvo prioritário; Windows 10 ainda precisa de validação.

## Créditos e licenças

Crunchy AI adapta o aplicativo e o denoiser do HushMic para Windows e mantém atribuição ao upstream. O HushMic está disponível sob MIT ou Apache-2.0; os modelos DPDFNet são Apache-2.0 e ONNX Runtime é MIT. Consulte os avisos de terceiros incluídos. VB-CABLE é um produto separado da VB-Audio e permanece sujeito aos termos do fornecedor.

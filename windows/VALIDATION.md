# Estado de validação — Windows

## Verificado

- A suíte do denoiser passou com 33 testes: unitários, API, latência do motor e paridade.
- O app passou com 10 testes, `cargo fmt --check` e Clippy sem avisos.
- O build Release x64 passou. O autoteste processou áudio sintético nos modos Qualidade e Leve e confirmou silêncio digital no modo Mudo. Esse autoteste não mede latência de áudio ponta a ponta.
- O pacote `CrunchyAI-Setup-x64.exe` foi gerado e validado com `--verify`; o pacote portátil contém somente o app, dependências documentadas e assets necessários. Nenhum driver VB-CABLE ou driver próprio é incluído.
- O logo foi carregado pelo app. O ícone Windows foi conferido no executável do app e no instalador.
- Em uma sessão real de Windows 11 com VB-CABLE instalado separadamente, os endpoints foram encontrados e o usuário confirmou que o retorno processado funcionou e que a supressão soou muito boa. Secure Boot permaneceu ativo. Esta é uma validação funcional manual, não um benchmark controlado.

## Ainda não verificado

- Uma sessão prolongada e controlada com duração, latência ponta a ponta e acompanhamento de underruns/descartes.
- Matriz com Discord/OBS, clientes simultâneos, troca e reconexão do microfone, suspensão/retomada e monitoramento local ligado/desligado.
- Instalação e remoção da versão final com marca Crunchy AI em um perfil limpo. A instalação/desinstalação é por usuário e não deve tocar no VB-CABLE.
- Compatibilidade com Windows 10 x64.

Secure Boot não precisa ser desativado para o desenho atual: o port usa o VB-CABLE oficial instalado separadamente e não instala driver próprio.

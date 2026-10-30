# Gerenciamento de cores

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

As telas mostram cores com luz vermelha, verde e azul; as impressoras misturam
tintas ciano, magenta, amarela e preta. O gerenciamento de cores usa **perfis de
cor** para descrever cada dispositivo, para que uma cor mantenha a aparência da
tela até a página impressa. As configurações ficam em
[[menu.tools > menu.tools.color_management]], a caixa de diálogo
[[color_mgmt.title]]. Nada muda até você clicar em [[dialog.ok]].

![A caixa de diálogo Configurações de gerenciamento de cores](../../img/color-color-management.png)

## Os perfis do desenho {#document}

A guia [[color_mgmt.tab_document]] define os perfis do desenho ativo.

- [[color_mgmt.current_profiles]] mostra os perfis RGB, CMYK e de escala de
  cinza que o desenho usa agora.
- Escolha como um novo perfil é aplicado:
  - [[color_mgmt.assign]]: os valores das cores ficam e a aparência muda para o
    novo perfil;
  - [[color_mgmt.convert]]: as cores CMYK são convertidas para terem a mesma
    aparência no novo perfil.
- Escolha os perfis [[color_mgmt.rgb_profile]], [[color_mgmt.cmyk_profile]] e
  [[color_mgmt.grayscale]]. Cada lista tem os perfis padrão e depois todos os
  perfis encontrados no seu computador. [[color_mgmt.load_profiles]] acrescenta
  arquivos de perfil (`.icc`, `.icm`).
- [[color_mgmt.primary_mode]]: CMYK ou RGB.
- [[color_mgmt.intent]]: como as cores que não podem ser mostradas são trazidas
  para dentro: [[color_mgmt.intent_relative]] (de início),
  [[color_mgmt.intent_perceptual]], [[color_mgmt.intent_saturation]] ou
  [[color_mgmt.intent_absolute]].

O perfil CMYK acompanha o desenho, também em arquivos `.cdr` a partir da versão
16.

## Padrões para novos desenhos {#default}

A guia [[color_mgmt.tab_default]] define o que os novos desenhos usam e como as
cores são convertidas.

- [[color_mgmt.preset]]: configurações prontas para a América do Norte, a
  Europa, o Japão ou trabalho para web e tela. [[color_mgmt.save_preset]] salva
  suas configurações com um [[color_mgmt.preset_name]];
  [[color_mgmt.delete_preset]] remove uma.
- Os perfis padrão [[color_mgmt.rgb_profile]], [[color_mgmt.cmyk_profile]] e
  [[color_mgmt.gray_profile]], o [[color_mgmt.primary_mode]] e a
  [[color_mgmt.intent]].
- [[color_mgmt.conversion]]:
  - [[color_mgmt.engine]]: [[color_mgmt.engine_icc]], que usa os perfis, ou
    [[color_mgmt.engine_builtin]];
  - [[color_mgmt.preserve_black]]: cores só de tinta preta continuam só de tinta
    preta ao serem convertidas;
  - [[color_mgmt.map_gray]] (ativado de início): cinzas convertidos em CMYK usam
    só tinta preta;
  - [[dialog.black_point]] (ativado de início): mantém separados os tons mais
    escuros.
- [[color_mgmt.open_policy]]: o que acontece com o perfil de um arquivo aberto,
  para RGB, CMYK e escala de cinza: [[color_mgmt.open_use_embedded]],
  [[color_mgmt.open_assign_default]] ou [[color_mgmt.open_convert_default]].
  [[color_mgmt.warn_mismatch_box]] e [[color_mgmt.warn_missing_box]] mostram uma
  mensagem na barra de status quando um arquivo tem outro perfil ou nenhum.
- [[color_mgmt.import_policy]]: [[color_mgmt.import_convert]] ou
  [[color_mgmt.import_assign]], para o que você importa ou cola.
- [[color_mgmt.spot_definition]]: se as cores especiais são definidas por
  [[color_mgmt.spot_lab]], [[color_mgmt.spot_cmyk]] ou
  [[color_mgmt.spot_rgb]].

## Visualizar as cores de impressão {#proof}

[[menu.view > menu.view.proof_colors]], ou o botão de prova na ponta direita da
barra de status, mostra o desenho como ele seria impresso: cores que as tintas
CMYK não conseguem imprimir ficam mais apagadas. O painel [[docker.color]]
também avisa sobre essas cores.

Sem perfis carregados, o TraceDraw converte as cores com o próprio modelo de
tintas de processo em papel revestido.

Para converter todas as cores da seleção ou do desenho em CMYK ou RGB com
esses perfis, veja [Converter cores para CMYK ou RGB](convert-colors.md).

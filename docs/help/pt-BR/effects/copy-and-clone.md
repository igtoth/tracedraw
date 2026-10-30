# Copiar e clonar efeitos

Em vez de configurar um efeito de novo, você pode copiá-lo de um objeto que já
o tem.

## Copiar um efeito {#copy}

1. Selecione os objetos que devem receber o efeito.
2. Escolha [[menu.object > menu.object.copy_effect]] e o tipo de efeito:
   [[menu.object.copy_shadow_from]], [[menu.object.copy_transparency_from]],
   [[menu.object.copy_envelope_from]], [[menu.object.copy_perspective_from]],
   [[menu.object.copy_distortion_from]], [[menu.object.copy_extrude_from]],
   [[menu.object.copy_contour_from]], [[menu.object.copy_blend_from]],
   [[menu.object.copy_lens_from]], [[menu.object.copy_bevel_from]] ou
   [[menu.object.copy_block_shadow_from]].
3. Clique no objeto de onde copiar. Um clique com o botão direito cancela.

O efeito é ajustado a cada objeto: um envelope ou uma perspectiva é esticado
para o tamanho dele, uma distorção mantém o centro no mesmo lugar dentro dele.
Quando o objeto clicado não tem esse efeito, nada muda e a barra de status
avisa. A cópia é um passo de desfazer.

## Clonar um efeito {#clone}

[[menu.object > menu.object.clone_effect]] copia a sombra
([[menu.object.clone_shadow_from]]) ou a transparência
([[menu.object.clone_transparency_from]]) de outro objeto e continua
acompanhando: mude a origem e os clones mudam também, até você editar o efeito
próprio de um clone.

[[menu.edit > menu.edit.clone]] faz um clone de um objeto inteiro, e
[[menu.edit.copy_properties_from]] (<kbd>Ctrl+Shift+A</kbd>) copia o
preenchimento, o contorno ou as propriedades de texto de outro objeto: marque o
que copiar (caneta de contorno, cor do contorno, preenchimento, propriedades do
texto), clique em **OK** e depois no objeto de onde copiar.

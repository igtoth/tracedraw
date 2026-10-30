# Cambiar el modo de color de los mapas de bits

[[menu.bitmaps > menu.bitmaps.mode]] convierte los mapas de bits seleccionados
a otro modo de color. Todos los mapas de bits seleccionados cambian en un solo
paso de deshacer, y se conserva la transparencia. Los modos con ajustes abren
un cuadro de diálogo con vistas previas de antes y después.

| Modo | Resultado |
|---|---|
| [[menu.bitmaps.mode_bw]] | solo blanco y negro (vea más abajo) |
| [[menu.bitmaps.mode_grayscale]] | tonos de gris |
| [[menu.bitmaps.mode_duotone]] | de una a cuatro tintas (vea más abajo) |
| [[menu.bitmaps.mode_paletted]] | como máximo 256 colores (vea más abajo) |
| [[menu.bitmaps.mode_rgb]] | los píxeles tal como están |
| [[menu.bitmaps.mode_lab]] | color Lab |
| [[menu.bitmaps.mode_cmyk]] | los píxeles tal como están; la gestión del color los convierte en la salida |

## Blanco y negro {#black-and-white}

![El cuadro de diálogo Blanco y negro](../../img/bitmaps-color-modes-black-and-white.png)

Elija el método de conversión:

- **Dibujo lineal**: blanco donde el gris está en el **umbral** o por encima
  (128 de forma predeterminada), negro en el resto;
- **Ordenado**: un patrón regular de puntos;
- **Semitono** (opción predeterminada): una trama de puntos redondos, de
  líneas o cuadrados, con un **ángulo** (45 grados) y un número de **líneas
  por pulgada** (60);
- **Distribución de cardinalidad**: una textura de aspecto aleatorio;
- **Jarvis**, **Stucki** y **Floyd-Steinberg**: difusión de error, los
  resultados más suaves para fotografías.

**Intensidad** (de 0 a 100, 50 de forma predeterminada) aclara u oscurece el
resultado.

## Duotono {#duotone}

![El cuadro de diálogo Duotono](../../img/bitmaps-color-modes-duotone.png)

Un duotono imprime una imagen en escala de grises con pocas tintas, lo que da
tonos ricos con pocos colores.

1. Elija el tipo: Monotono, Duotono (opción predeterminada), Tritono o
   Cuadritono, para una a cuatro tintas.
2. Haga clic en una tinta para cambiar su color.
3. Edite su curva: en horizontal está el nivel de gris (el negro a la
   izquierda), en vertical cuánta tinta se imprime. Arrastre en un espacio
   vacío para añadir un punto, arrastre un punto para moverlo, y haga doble
   clic en él o arrástrelo fuera de la cuadrícula para quitarlo. **Mostrar
   todo** dibuja también las curvas de las demás tintas.
4. **Guardar** y **Cargar** guardan las tintas y las curvas en un archivo.

## Con paleta {#paletted}

![El cuadro de diálogo Con paleta](../../img/bitmaps-color-modes-paletted.png)

- **Paleta**: Colores uniformes, VGA estándar (16 colores), Adaptable y
  Optimizada (los propios colores de la imagen, reducidos al número de
  **colores** que indique, hasta 256), Escala de grises, Sistema o
  Personalizada (la paleta del documento).
- **Tramado**: Ninguno, Ordenado, Jarvis, Stucki o Floyd-Steinberg, con la
  intensidad del tramado.

La paleta resultante aparece debajo de los ajustes.

## Colorear mapas de bits en blanco y negro {#monochrome}

Con solo mapas de bits de dos colores seleccionados, haga clic en un color de
la paleta para colorear su fondo, y haga clic con el botón derecho para
colorear su primer plano.

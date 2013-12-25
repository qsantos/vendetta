#ifndef GRAPHICS_H
#define GRAPHICS_H

#include <SFML/Graphics.h>

typedef struct graphics graphics_t;

struct graphics
{
	sfRenderWindow* render;
	sfBool          hasFocus;
	sfFont*         font;

	sfView* world_view;

	char**      filenames;
	sfTexture** textures;

	size_t     n_sprites;
	size_t     a_sprites;
	sfSprite** sprites;
};

graphics_t* graphics_init(void);
void        graphics_exit(graphics_t* g);

int  graphics_spriteForImg(graphics_t* g, const char* filename);

void graphics_drawProgressBar(graphics_t* g, float x, float y, float w, float h, float p);

#endif

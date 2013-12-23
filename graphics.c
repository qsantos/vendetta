#include "graphics.h"

#include <string.h>

#include "util.h"

#define HT_MAX_TEXTURES 128

static unsigned int find(graphics_t* g, const char* str);

graphics_t* graphics_init(void)
{
	graphics_t* g = CALLOC(graphics_t, 1);

	sfVideoMode mode = {800, 600, 32};
	g->render = sfRenderWindow_create(mode, "Vendetta 0.2", sfResize | sfClose, NULL);
	if (g->render == NULL)
		exit(1);
	sfRenderWindow_setMouseCursorVisible(g->render, sfFalse);

	g->filenames = CALLOC(char*, HT_MAX_TEXTURES);
	memset(g->filenames, 0, sizeof(char*) * HT_MAX_TEXTURES);

	g->textures = CALLOC(sfTexture*, HT_MAX_TEXTURES);

	g->n_sprites = 0;
	g->a_sprites = 0,
	g->sprites = NULL;

	return g;
}

void graphics_exit(graphics_t* g)
{
	for (size_t i = 0; i < g->n_sprites; i++)
	{
		sfSprite_destroy(g->sprites[i]);
	}
	for (size_t i = 0; i < HT_MAX_TEXTURES; i++)
	{
		sfTexture_destroy(g->textures[i]);
		free(g->filenames[i]);
	}

	sfRenderWindow_destroy(g->render);
	free(g);
}

int graphics_spriteForImg(graphics_t* g, const char* filename)
{
	int id = find(g, filename);
	if (g->filenames[id] == NULL)
	{
		g->filenames[id] = strdup(filename);

		sfTexture* texture = sfTexture_createFromFile(filename, NULL);
		g->textures[id] = texture;

		if (texture == NULL)
			exit(1);
	}

	sfSprite* sprite = sfSprite_create();
	sfSprite_setTexture(sprite, g->textures[id], sfTrue);

	if (g->n_sprites == g->a_sprites)
	{
		g->a_sprites = g->a_sprites ? 2*g->a_sprites : 1;
		g->sprites = CREALLOC(g->sprites, sfSprite*, g->a_sprites);
	}

	g->sprites[g->n_sprites] = sprite;
	return g->n_sprites++;
}

static unsigned int hash(const char* str)
{
   unsigned int h = 0x4e67c6a7;
   for (; *str; str++)
      h ^= (h << 5) + (*str) + (h >> 2);
   return h % HT_MAX_TEXTURES;
}
static unsigned int find(graphics_t* g, const char* str)
{
	unsigned int id;
	for (id = hash(str); g->filenames[id]; id++)
		if (strcmp(g->filenames[id], str) == 0)
			return id;
	return id;
}

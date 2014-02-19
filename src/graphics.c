/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "graphics.h"

#include <stdlib.h>

#include "version.h"
#include "mem.h"
#include "string.h"

#define HT_MAX_TEXTURES (16*1024)

static unsigned int find(graphics_t* g, const char* str);

void graphics_init(graphics_t* g)
{
	sfVideoMode mode = {1024, 768, 32};
	g->render = sfRenderWindow_create(mode, "Vendetta " VERSION, sfResize | sfClose, NULL);
	if (g->render == NULL)
		exit(1);
	sfRenderWindow_setMouseCursorVisible(g->render, sfFalse);

	g->font = sfFont_createFromFile("data/DejaVuSans.ttf");
	if (g->font == NULL)
		exit(1);

	g->filenames = CALLOC(char*, HT_MAX_TEXTURES);
	memset(g->filenames, 0, sizeof(char*) * HT_MAX_TEXTURES);

	g->textures = CALLOC(sfTexture*, HT_MAX_TEXTURES);
	memset(g->textures, 0, sizeof(sfTexture*) * HT_MAX_TEXTURES);

	g->n_sprites = 0;
	g->a_sprites = 0,
	g->sprites = NULL;

	g->step = 0;
	g->fps  = 0;
}

void graphics_exit(graphics_t* g)
{
	for (size_t i = 0; i < g->n_sprites; i++)
		sfSprite_destroy(g->sprites[i]);
	free(g->sprites);

	for (size_t i = 0; i < HT_MAX_TEXTURES; i++)
	{
		sfTexture_destroy(g->textures[i]);
		free(g->filenames[i]);
	}
	free(g->textures);
	free(g->filenames);

	sfFont_destroy(g->font);

	sfRenderWindow_destroy(g->render);
}

sfTexture* graphics_loadImage(graphics_t* g, const char* filename)
{
	int id = find(g, filename);
	if (g->filenames[id] == NULL)
	{
		sfTexture* texture = sfTexture_createFromFile(filename, NULL);
		if (texture == NULL)
		{
			fprintf(stderr, "Failed to load texture '%s'\n", filename);
			exit(1);
		}

		g->filenames[id] = strdup(filename);
		g->textures [id] = texture;
	}
	return g->textures[id];

}

int graphics_spriteId(graphics_t* g, const char* filename)
{
	sfTexture* texture = graphics_loadImage(g, filename);

	sfSprite* sprite = sfSprite_create();
	sfSprite_setTexture(sprite, texture, sfTrue);

	if (g->n_sprites == g->a_sprites)
	{
		g->a_sprites = g->a_sprites ? 2*g->a_sprites : 1;
		g->sprites = CREALLOC(g->sprites, sfSprite*, g->a_sprites);
	}

	g->sprites[g->n_sprites] = sprite;
	return g->n_sprites++;
}

sfSprite* graphics_sprite(graphics_t* g, const char* filename)
{
	int id = graphics_spriteId(g, filename);
	return g->sprites[id];
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
	unsigned int start = hash(str);
	unsigned int id = start;
	while (g->filenames[id] != NULL && strcmp(g->filenames[id], str) != 0)
	{
		id = (id+1) % HT_MAX_TEXTURES;
		if (id == start)
		{
			fprintf(stderr, "Too many textures\n");
			exit(1);
		}
	}
	return id;
}

#ifdef __WIN32__
#include <windows.h>
#include <winnls.h>
#endif
void sfText_setUTF8(sfText* text, const char* string)
{
	sfUint32 buf32[1024];
#ifdef __WIN32__
	wchar_t buf16[1024];
	MultiByteToWideChar(CP_UTF8, 0, string, -1, buf16, 1024);
	size_t n = wcslen(buf16)+1;
	for (size_t i = 0; i < n; i++)
		buf32[i] = buf16[i];
#else
	mbstowcs((wchar_t*) buf32, string, 1024);
#endif
	sfText_setUnicodeString(text, buf32);
}

char sfText_contains(sfText* text, sfVector2f point)
{
	sfFloatRect rect = sfText_getGlobalBounds(text);
	return sfFloatRect_contains(&rect, point.x, point.y);
}

char sfSprite_contains(sfSprite* sprite, sfVector2f point)
{
	sfFloatRect rect = sfSprite_getGlobalBounds(sprite);
	return sfFloatRect_contains(&rect, point.x, point.y);
}

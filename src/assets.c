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

#include "assets.h"

#include <stdlib.h>

#include "mem.h"
#include "string.h"

#define HT_MAX_TEXTURES (16*1024)

static unsigned int find(assets_t* a, const char* str);

void assets_init(assets_t* a)
{
	a->font = sfFont_createFromFile("data/DejaVuSans.ttf");
	if (a->font == NULL)
		exit(1);

	a->filenames = CALLOC(char*, HT_MAX_TEXTURES);
	memset(a->filenames, 0, sizeof(char*) * HT_MAX_TEXTURES);

	a->textures = CALLOC(sfTexture*, HT_MAX_TEXTURES);
	memset(a->textures, 0, sizeof(sfTexture*) * HT_MAX_TEXTURES);

	a->n_sprites = 0;
	a->a_sprites = 0,
	a->sprites = NULL;
}

void assets_exit(assets_t* a)
{
	for (size_t i = 0; i < a->n_sprites; i++)
		sfSprite_destroy(a->sprites[i]);
	free(a->sprites);

	for (size_t i = 0; i < HT_MAX_TEXTURES; i++)
	{
		sfTexture_destroy(a->textures[i]);
		free(a->filenames[i]);
	}
	free(a->textures);
	free(a->filenames);

	sfFont_destroy(a->font);
}

sfTexture* assets_loadImage(assets_t* a, const char* filename)
{
	int id = find(a, filename);
	if (a->filenames[id] == NULL)
	{
		sfTexture* texture = sfTexture_createFromFile(filename, NULL);
		if (texture == NULL)
		{
			fprintf(stderr, "Failed to load texture '%s'\n", filename);
			exit(1);
		}

		a->filenames[id] = strdup(filename);
		a->textures [id] = texture;
	}
	return a->textures[id];

}

int assets_spriteId(assets_t* a, const char* filename)
{
	sfTexture* texture = assets_loadImage(a, filename);

	sfSprite* sprite = sfSprite_create();
	sfSprite_setTexture(sprite, texture, sfTrue);

	if (a->n_sprites == a->a_sprites)
	{
		a->a_sprites = a->a_sprites ? 2*a->a_sprites : 1;
		a->sprites = CREALLOC(a->sprites, sfSprite*, a->a_sprites);
	}

	a->sprites[a->n_sprites] = sprite;
	return a->n_sprites++;
}

sfSprite* assets_sprite(assets_t* a, const char* filename)
{
	int id = assets_spriteId(a, filename);
	return a->sprites[id];
}

static unsigned int hash(const char* str)
{
	unsigned int h = 0x4e67c6a7;
	for (; *str; str++)
		h ^= (h << 5) + (*str) + (h >> 2);
	return h % HT_MAX_TEXTURES;
}
static unsigned int find(assets_t* a, const char* str)
{
	unsigned int start = hash(str);
	unsigned int id = start;
	while (a->filenames[id] != NULL && strcmp(a->filenames[id], str) != 0)
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

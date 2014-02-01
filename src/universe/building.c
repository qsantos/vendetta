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

#include "building.h"

#include <stdlib.h>
#include <stdio.h>

#include "../mem.h"

void kindOf_building_init(kindOf_building_t* b)
{
	b->name = NULL;
	b->width = 0;
	b->height = 0;
	b->sprite = 0;
	b->n_sprites = 0;
	b->door_dx = 0;
	b->door_dy = 0;

	b->button_sprite = -1;
	b->button_index = -1;

	transform_init(&b->build);
	transform_init(&b->make);

	b->n_items = 0;
	b->items = NULL;
}

void kindOf_building_exit(kindOf_building_t* b)
{
	for (size_t i = 0; i < b->n_items; i++)
		transform_exit(&b->items[i]);
	free(b->items);
	transform_exit(&b->make);
	transform_exit(&b->build);
	free(b->name);
}

void kindOf_building_sprite(kindOf_building_t* b, graphics_t* g, const char* filename, int n_sprites)
{
	char s[1024];
	snprintf(s, 1024, "data/buildings/%s", filename);
	int id = graphics_spriteId(g, s);

	b->sprite = id;
	b->n_sprites = n_sprites;

	sfIntRect rect = sfSprite_getTextureRect(g->sprites[id]);
	b->width  = rect.width;
	b->height = rect.height / n_sprites;
}

void kindOf_building_button(kindOf_building_t* b, graphics_t* g, const char* filename, int idx)
{
	char s[1024];
	snprintf(s, 1024, "data/%s", filename);
	int id = graphics_spriteId(g, s);

	b->button_sprite = id;
	b->button_index = idx;
}

int kindOf_building_newItem(kindOf_building_t* b)
{
	b->items = CREALLOC(b->items, transform_t, b->n_items+1);
	transform_init(&b->items[b->n_items]);
	return b->n_items++;
}

transform_t* kindOf_building_available(kindOf_building_t* b, int id, int is_item)
{
	if (transform_is_res(&b->make, id, is_item) >= 0)
		return &b->make;

	for (size_t i = 0; i < b->n_items; i++)
		if (transform_is_res(&b->items[i], id, is_item) >= 0)
			return &b->items[i];

	return NULL;
}

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

#include "draw.h"

#include <stdlib.h>
#include <math.h>

#include "draw_tilemap.h"

void draw_object(graphics_t* g, object_t* o, sfSprite* sprite)
{
	sfVector2f pos = {o->x - o->w/2, o->y - o->h};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_character(graphics_t* g, character_t* c)
{
	if (c == NULL)
		return;

	if (c->inBuilding != NULL)
		return;

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "character.png");
		sprite = g->sprites[id];
	}

	int step = floor(c->step);
	if (step >= 3)
		step = 1;

	sfIntRect  rect = {24*step, 32*c->dir, 24, 32};
	sfVector2f pos  = {c->o.x - c->o.w/2, c->o.y - c->o.h};

	if (c->inWater)
	{
		pos.y += 12;
		rect.height -= 12;
	}

	sfSprite_setTextureRect(sprite, rect);
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_mine(graphics_t* g, mine_t* m)
{
	if (m == NULL)
		return;

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "mines.png");
		sprite = g->sprites[id];
	}

	int t = m->t->id;
	sfIntRect rect = {32*t, 32*0, 32, 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &m->o, sprite);
}

void draw_building(graphics_t* g, building_t* b)
{
	if (b == NULL)
		return;

	int step;
	int n = b->t->n_sprites;
	float p = b->build_progress;

	if (n <= 1)
		step = 0;
	else
		step = floor(p * (n-2));

	sfSprite* sprite = g->sprites[b->t->sprite];
	sfIntRect rect = {0, b->o.h*step, b->o.w, b->o.h};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &b->o, sprite);

	graphics_drawProgressBar(g, b->o.x - b->o.w/2, b->o.y+1, b->o.w, 5, b->build_progress);
}

void draw_world(graphics_t* g, world_t* w)
{
	draw_tilemap(g, w);

	for (size_t i = 0; i < w->n_buildings; i++)
		draw_building(g, w->buildings[i]);

	for (size_t i = 0; i < w->n_mines; i++)
		draw_mine(g, &w->mines[i]);

	for (size_t i = 0; i < w->n_characters; i++)
		draw_character(g, &w->characters[i]);
}

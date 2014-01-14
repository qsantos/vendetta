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

void draw_object(graphics_t* g, object_t* o, sfSprite* sprite)
{
	sfVector2f pos = {o->x - o->w/2, o->y - o->h};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_character(graphics_t* g, character_t* c)
{
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

	sfIntRect rect = {24*step, 32*c->dir, 24, c->inWater ? 20 : 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &c->o, sprite);
}

void draw_mine(graphics_t* g, mine_t* m)
{
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
	int step;
	int n = b->t->n_sprites;
	float p = b->build_progress;

	if (n <= 1)
		step = 0;
	else
		step = floor((float)(n-2) * p);

	sfSprite* sprite = g->sprites[b->t->sprite];
	sfIntRect rect = {0, b->o.h*step, b->o.w, b->o.h};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &b->o, sprite);

	graphics_drawProgressBar(g, b->o.x - b->o.w/2, b->o.y+1, b->o.w, 5, b->build_progress);
}

void draw_world(graphics_t* g, world_t* w)
{
	static sfRenderStates states = {sfBlendAlpha, {{1,0,0,0,1,0,0,0,1}}, NULL, NULL};
	static sfVertexArray* array = NULL;
	if (array == NULL)
	{
		sfTexture* texture = sfTexture_createFromFile("lands.png", NULL);

		int x = w->tilesx;
		int y = w->tilesy;
		array = sfVertexArray_create();
		sfVertexArray_setPrimitiveType(array, sfQuads);
		sfVertexArray_resize(array, x*y*4);

		for (int i = 0; i < x; i++)
			for (int j = 0; j < y; j++)
			{
				sfVertex* v = sfVertexArray_getVertex(array, (j*x+i)*4);

				float a = (i-x/2)*16;
				float b = (j-y/2)*16;
				v[0].position = (sfVector2f){a+ 0,b+ 0};
				v[1].position = (sfVector2f){a+16,b+ 0};
				v[2].position = (sfVector2f){a+16,b+16};
				v[3].position = (sfVector2f){a+ 0,b+16};

				for (int k = 0; k < 4; k++)
					v[k].color = sfWhite;

				int t = TERRAIN(w,i,j);
				a = 16*(t%16);
				b = 16*(t/16);
				v[0].texCoords = (sfVector2f){a+ 0.01,b+ 0.01};
				v[1].texCoords = (sfVector2f){a+15.99,b+ 0.01};
				v[2].texCoords = (sfVector2f){a+15.99,b+15.99};
				v[3].texCoords = (sfVector2f){a+ 0.01,b+15.99};
			}

		states.texture = texture;
	}
	static int water_step = 0;
	int cur_step = (int)g->step % 4;
	if (cur_step != water_step)
	{
		water_step = cur_step;

		int x = w->tilesx;
		int y = w->tilesy;
		for (int i = 0; i < x; i++)
		for (int j = 0; j < y; j++)
		{
			int t = TERRAIN(w,i,j);
			if (!(160 <= t && t < 176))
				continue;
			t += 16*(water_step == 3 ? 1 : water_step);
			float a = 16*(t%16);
			float b = 16*(t/16);
			sfVertex* v = sfVertexArray_getVertex(array, (j*x+i)*4);
			v[0].texCoords = (sfVector2f){a+ 0.01,b+ 0.01};
			v[1].texCoords = (sfVector2f){a+15.99,b+ 0.01};
			v[2].texCoords = (sfVector2f){a+15.99,b+15.99};
			v[3].texCoords = (sfVector2f){a+ 0.01,b+15.99};
		}
	}

	sfRenderWindow_drawVertexArray(g->render, array, &states);

	for (size_t i = 0; i < w->n_buildings; i++)
		draw_building(g, w->buildings[i]);

	for (size_t i = 0; i < w->n_mines; i++)
		draw_mine(g, &w->mines[i]);

	for (size_t i = 0; i < w->n_characters; i++)
		draw_character(g, &w->characters[i]);
}

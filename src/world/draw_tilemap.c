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

#include "draw_tilemap.h"

#include <math.h>

void tilemap_update(sfVertexArray* array, chunk_t* c)
{
	for (int i = 0; i < c->rows; i++)
		for (int j = 0; j < c->cols; j++)
		{
			sfVertex* v = sfVertexArray_getVertex(array, (j*c->rows+i)*4);

			float a = c->o.x + (i-c->rows/2)*16;
			float b = c->o.y + (j-c->cols  )*16;
			v[0].position = (sfVector2f){a+ 0,b+ 0};
			v[1].position = (sfVector2f){a+16,b+ 0};
			v[2].position = (sfVector2f){a+16,b+16};
			v[3].position = (sfVector2f){a+ 0,b+16};

			for (int k = 0; k < 4; k++)
				v[k].color = sfWhite;

			int t = TERRAIN(c,i,j);
			a = 16*(t%16);
			b = 16*(t/16);
			v[0].texCoords = (sfVector2f){a+ 0.01,b+ 0.01};
			v[1].texCoords = (sfVector2f){a+15.99,b+ 0.01};
			v[2].texCoords = (sfVector2f){a+15.99,b+15.99};
			v[3].texCoords = (sfVector2f){a+ 0.01,b+15.99};
		}
}

void tilemap_water(sfVertexArray* array, chunk_t* c)
{
	int step = c->water_step == 3 ? 1 : c->water_step;
	for (int i = 0; i < c->rows; i++)
		for (int j = 0; j < c->cols; j++)
		{
			int t = TERRAIN(c,i,j);
			if (!(160 <= t && t < 176))
				continue;
			t += 16*step;
			float a = 16*(t%16);
			float b = 16*(t/16);
			sfVertex* v = sfVertexArray_getVertex(array, (j*c->rows+i)*4);
			v[0].texCoords = (sfVector2f){a+ 0.01,b+ 0.01};
			v[1].texCoords = (sfVector2f){a+15.99,b+ 0.01};
			v[2].texCoords = (sfVector2f){a+15.99,b+15.99};
			v[3].texCoords = (sfVector2f){a+ 0.01,b+15.99};
		}
}

void draw_tilemap(graphics_t* g, world_t* w)
{
	static sfRenderStates states = {sfBlendAlpha, {{1,0,0,0,1,0,0,0,1}}, NULL, NULL};
	if (states.texture == NULL)
		states.texture = graphics_loadImage(g, "lands.png");

	for (size_t i = 0; i < w->n_chunks; i++)
	{
		chunk_t* c = &w->chunks[i];

		sfVector2f x = sfView_getCenter(g->world_view);
		sfVector2f s = sfView_getSize(g->world_view);
		object_t o = {O_NONE, x.x, x.y+s.y/2, s.x, s.y};

		if (!object_overlaps(&c->o, &o))
			continue;

		sfVertexArray* array = c->array;
		int cur_step = floor(g->step);
		cur_step %= 4;
		if (cur_step != c->water_step)
		{
			c->water_step = cur_step;
			tilemap_water(array, c);
		}

		sfRenderWindow_drawVertexArray(g->render, array, &states);
	}
}

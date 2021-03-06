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

#include "chunk.h"

#include <stdlib.h>
#include <string.h>

#include "../mem.h"

void chunk_init(chunk_t* c, world_t* w, float x, float y, int rows, int cols)
{
	c->o.t = O_CHUNK;
	c->o.x = x;
	c->o.y = y;
	c->o.w = rows * TILE_SIZE;
	c->o.h = cols * TILE_SIZE;

	c->w = w;

	c->rows = rows;
	c->cols = cols;
	c->lands = CALLOC(short, rows*cols);

	c->water_step = 0;
	c->array = sfVertexArray_create();
	sfVertexArray_setPrimitiveType(c->array, sfQuads);
	sfVertexArray_resize(c->array, rows*cols*4);

	c->n_mines = 0;
	c->mines = NULL;

	c->n_buildings = 0;
	c->buildings = NULL;
}

void chunk_exit(chunk_t* c)
{
	free(c->mines);
	sfVertexArray_destroy(c->array);
	free(c->lands);
}

void chunk_update(chunk_t* c)
{
	for (int i = 0; i < c->rows; i++)
		for (int j = 0; j < c->cols; j++)
		{
			sfVertex* v = sfVertexArray_getVertex(c->array, (i*c->cols+j)*4);

			float a = c->o.x + (j-.5*c->cols)*16;
			float b = c->o.y + (i-   c->rows)*16;
			v[0].position = (sfVector2f){a+ 0,b+ 0};
			v[1].position = (sfVector2f){a+16,b+ 0};
			v[2].position = (sfVector2f){a+16,b+16};
			v[3].position = (sfVector2f){a+ 0,b+16};

			for (int k = 0; k < 4; k++)
				v[k].color = sfWhite;

			int t = LAND(c,i,j);
			a = 16*(t%16);
			b = 16*(t/16);
			v[0].texCoords = (sfVector2f){a+ 0.01,b+ 0.01};
			v[1].texCoords = (sfVector2f){a+15.99,b+ 0.01};
			v[2].texCoords = (sfVector2f){a+15.99,b+15.99};
			v[3].texCoords = (sfVector2f){a+ 0.01,b+15.99};
		}
}

void chunk_updwtr(chunk_t* c)
{
	int step = c->water_step == 3 ? 1 : c->water_step;
	for (int i = 0; i < c->rows; i++)
		for (int j = 0; j < c->cols; j++)
		{
			int t = LAND(c,i,j);
			if (t/16 != 10)
				continue;
			t += 16*step;
			float a = 16*(t%16);
			float b = 16*(t/16);
			sfVertex* v = sfVertexArray_getVertex(c->array, (i*c->cols+j)*4);
			v[0].texCoords = (sfVector2f){a+ 0.01,b+ 0.01};
			v[1].texCoords = (sfVector2f){a+15.99,b+ 0.01};
			v[2].texCoords = (sfVector2f){a+15.99,b+15.99};
			v[3].texCoords = (sfVector2f){a+ 0.01,b+15.99};
		}
}

void chunk_pushMine(chunk_t* c, mine_t* m)
{
	if (c->n_mines != 0 && m == c->mines[c->n_mines-1])
		return;

	c->mines = CREALLOC(c->mines, mine_t*, c->n_mines+1);
	c->mines[c->n_mines++] = m;
}

void chunk_pushBuilding(chunk_t* c, building_t* b)
{
	if (c->n_buildings != 0 && b == c->buildings[c->n_buildings-1])
		return;

	c->buildings = CREALLOC(c->buildings, building_t*, c->n_buildings+1);
	c->buildings[c->n_buildings++] = b;
}

void chunk_delBuilding(chunk_t* c, uuid_t uuid)
{
	for (size_t i = 0; i < c->n_buildings; i++)
	{
		if (uuid == c->buildings[i]->o.uuid)
		{
			memmove(c->buildings + i, c->buildings + i + 1, sizeof(building_t*) * (c->n_buildings - i - 1));
			c->n_buildings--;
			break;
		}
	}
}

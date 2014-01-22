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

#include "../mem.h"
#include "../rand.h"
#include "draw_tilemap.h"

void chunk_init(chunk_t* c, float x, float y, int rows, int cols)
{
	c->o.t = O_CHUNK;
	c->o.x = x;
	c->o.y = y;
	c->o.w = rows * TILE_SIZE;
	c->o.h = cols * TILE_SIZE;

	c->rows = rows;
	c->cols = cols;
	c->lands = CALLOC(short, rows*cols);

	static const float land_probas[] = {0.8, 0.05, 0.05, 0.04, 0.01, 0,0,0,0,0,0.05};
	for (int i = 0; i < rows; i++)
		for (int j = 0; j < cols; j++)
			TERRAIN(c, i, j) = 16 * rnd_pick(land_probas);

	c->array = sfVertexArray_create();
	sfVertexArray_setPrimitiveType(c->array, sfQuads);
	sfVertexArray_resize(c->array, rows*cols*4);

	tilemap_update(c->array, c);
}

void chunk_exit(chunk_t* c)
{
	sfVertexArray_destroy(c->array);
	free(c->lands);
}

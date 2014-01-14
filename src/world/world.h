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

#ifndef W_WORLD_H
#define W_WORLD_H

typedef struct world world_t;

#include "../universe/universe.h"
#include "character.h"
#include "mine.h"
#include "object.h"
#include "building.h"

#define TILE_SIZE 16
#define TERRAIN(W,I,J) ((W)->terrain[(J)*(W)->tilesx + (I)])
#define TERRAINI(W,X)  ((int)floor((X) / TILE_SIZE + 0.5*(W)->tilesx))
#define TERRAINJ(W,Y)  ((int)floor((Y) / TILE_SIZE + 0.5*(W)->tilesy))

struct world
{
	object_t o;

	universe_t* universe;

	int tilesx;
	int tilesy;
	short* terrain;

	size_t n_characters;
	character_t* characters;

	size_t n_mines;
	mine_t* mines;

	size_t n_buildings;
	size_t a_buildings;
	building_t** buildings;
};

world_t* world_init(universe_t* u, int w, int h);
void     world_exit(world_t* w);

void world_randMine(world_t* w, mine_t* m);

void world_doRound(world_t* w, float duration);

int         world_landAt      (world_t* w, float x, float y);
object_t*   world_objectAt    (world_t* w, float x, float y);
mine_t*     world_findMine    (world_t* w, float x, float y, kindOf_mine_t* t);
building_t* world_findBuilding(world_t* w, float x, float y, kindOf_building_t* t);
char        world_canBuild    (world_t* w, float x, float y, kindOf_building_t* b);
building_t* world_addBuilding (world_t* w, kindOf_building_t* b, float x, float y);

#endif

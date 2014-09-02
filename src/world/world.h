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

#include "../settings.h"
#include "../universe/universe.h"
#include "chunk.h"
#include "event.h"
#include "character.h"
#include "object.h"
#include "building.h"
#include "pool.h"

#define CHUNK(W,I,J) (&(W)->chunks[(I)*(W)->chunk_cols+(J)])

struct world
{
	object_t o;

	settings_t* settings;
	universe_t* universe;

	unsigned int seed;
	int cols;
	int rows;
	int chunk_cols;
	int chunk_rows;
	size_t n_chunks;
	chunk_t* chunks;

	// on-going events
	evtList_t events;

	pool_t objects;
};

#include <stdio.h>

#include "../game.h"

void world_init(world_t* w, game_t* g);
void world_exit(world_t* w);

chunk_t* world_chunkXY(world_t* w, float x, float y);

short* world_landXY   (world_t* w, float x, float y);
short  world_getLandXY(world_t* w, float x, float y);
void   world_setLandXY(world_t* w, float x, float y, short l);
short* world_landIJ   (world_t* w, int i, int j);
short  world_getLandIJ(world_t* w, int i, int j);
void   world_setLandIJ(world_t* w, int i, int j, short l);

void world_doRound(world_t* w, float duration);

object_t* world_objectAt(world_t* w, float x, float y, object_t* ignore);

mine_t*      world_findMine           (world_t* w, float x, float y, kindOf_mine_t* t);
building_t*  world_findBuilding       (world_t* w, float x, float y, kindOf_building_t* t);
building_t*  world_findEnnemyBuilding (world_t* w, float x, float y, character_t* c);
building_t*  world_findSale           (world_t* w, float x, float y, char is_item, int id);
character_t* world_findEnnemyCharacter(world_t* w, character_t* c);

mine_t*     world_addMine     (world_t* w, float x, float y, kindOf_mine_t* t);

char        world_canBuild    (world_t* w, float x, float y, kindOf_building_t* t);
building_t* world_addBuilding (world_t* w, float x, float y, kindOf_building_t* t, character_t* c);
void        world_delBuilding (world_t* w, building_t* b);

// world_gen.c
void world_genmap  (world_t* w, unsigned int seed);
void world_start   (world_t* w);
void world_randMine(world_t* w, int type);

// world_save.c
void world_save(world_t* w, FILE* f);
void world_load(world_t* w, FILE* f);

#endif

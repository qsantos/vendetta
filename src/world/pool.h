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

#ifndef W_POOL_H
#define W_POOL_H

typedef struct pool pool_t;

#include "object.h"

struct pool
{
	size_t n_objects;
	size_t a_objects;
	object_t** objects;
};

void pool_init(pool_t* p);
void pool_exit(pool_t* p);

object_t* pool_new(pool_t* p, uuid_t uuid, size_t size);
object_t* pool_get(pool_t* p, uuid_t uuid);
void      pool_del(pool_t* p, object_t* a);

#include "character.h"
#include "building.h"

character_t* character_new(pool_t* p, uuid_t uuid);
character_t* character_get(pool_t* p, uuid_t uuid);

mine_t* mine_new(pool_t* p, uuid_t uuid);
mine_t* mine_get(pool_t* p, uuid_t uuid);

building_t* building_new(pool_t* p, uuid_t uuid);
building_t* building_get(pool_t* p, uuid_t uuid);

#endif

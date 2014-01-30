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

#include "pool.h"

#include <string.h>
#include <stdlib.h>

#include "../mem.h"

void pool_init(pool_t* p)
{
	p->n_objects = 0;
	p->a_objects = 0;
	p->objects = NULL;
}

void pool_exit(pool_t* p)
{
	free(p->objects);
}

uuid_t pool_new(pool_t* p, size_t size)
{
	if (p->n_objects >= p->a_objects)
	{
		p->a_objects = p->a_objects == 0 ? 1 : 2*p->a_objects;
		p->objects = CREALLOC(p->objects, object_t*, p->a_objects);
		memset(p->objects + p->n_objects, 0, (p->a_objects - p->n_objects)*sizeof(object_t*));
	}

	uuid_t uuid = p->n_objects++;
	object_t* o = (object_t*) CALLOC(char, size);
	o->uuid = uuid;
	p->objects[uuid] = o;
	return uuid;
}

object_t* pool_get(pool_t* p, uuid_t uuid)
{
	if (!(0 <= uuid && uuid < (ssize_t) p->n_objects))
		return NULL;
	return p->objects[uuid];
}

void pool_del(pool_t* p, object_t* a)
{
	if (a == NULL)
		return;
	p->objects[a->uuid] = NULL;
	free(a);
}

#define SPECIALIZED(T,K) \
T##_t* T##_new(pool_t* p) \
{ \
	uuid_t uuid = pool_new(p, sizeof(T##_t)); \
	return (T##_t*) pool_get(p, uuid); \
} \
T##_t* T##_get(pool_t* p, uuid_t uuid) \
{ \
	T##_t* r = (T##_t*) pool_get(p, uuid); \
	if (r == NULL || r->o.t != K) return NULL; \
	return r; \
}

SPECIALIZED(character, O_CHARACTER)
SPECIALIZED(building,  O_BUILDING)

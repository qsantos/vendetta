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

#include "ai.h"

#include <string.h>

#include "util.h"

void ai_init(ai_t* ai)
{
	ai->building = -1;
}

void ai_exit(ai_t* ai)
{
	(void) ai;
}

void ai_load(ai_t* ai, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open AI file '%s'\n", filename);
		exit(1);
	}

	int age = 0;

	char* line = NULL;
	size_t n_line = 0;
	while (getline(&line, &n_line, f) > 0)
	{
		int val = 0;
		char* strval = strchr(line, '=');
		if (strval != NULL)
		{
			*strval = 0;
			val = strtol(strval+1, NULL, 0);
		}

		int id = 0;
		char* strid = strchr(line, '_');
		if (strid != NULL)
		{
			*strid = 0;
			id = strtol(strid+1, NULL, 0);
		}

		char* key = line;
		(void) val;

		if (strcmp(key, "Age") == 0)
		{
			age = id;
		}

		if (age != 1)
			continue;

		if (strcmp(key, "Batiment") == 0)
		{
			ai->building = id - 1;
		}
	}

	fclose(f);
}


char ai_gather(character_t* c, int id, float amount)
{
	if (c->inventory.materials[id] >= amount)
		return 0;

	universe_t* u = c->universe;
	for (size_t i = 0; i < u->n_mines; i++)
	{
		kindOf_mine_t* t = &u->mines[i];
		if (transform_is_res(&t->harvest, id, 0))
		{
			character_goMine(c, t);
			return 1;
		}
	}

	for (size_t i = 0; i < u->n_buildings; i++)
	{
		kindOf_building_t* t = &u->buildings[i];
		if (transform_is_res(&t->make, id, 0))
		{
			ai_build(c, i);
			return 1;
		}
	}

	return 1;
}
char ai_build(character_t* c, int id)
{
	kindOf_building_t* t = &c->universe->buildings[id];
	building_t* b = world_findBuilding(c->world, c->o.x, c->o.y, t);
	if (b != NULL)
	{
		c->go_o = &b->o;
		return 0;
	}

	transform_t* tr = &t->build;
	for (int i = 0; i < tr->n_req; i++)
		if (ai_gather(c, tr->req[i].id, tr->req[i].amount))
			return 1;

	character_buildAuto(c, t);
	return 1;
}
void ai_do(ai_t* ai, character_t* c)
{
	ai_build(c, ai->building);
}

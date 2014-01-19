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

#include <stdlib.h>
#include <stdio.h>

#include "string.h"

void ai_init(ai_t* ai)
{
	transform_init(&ai->inventory);
	ai->building = -1;
}

void ai_exit(ai_t* ai)
{
	transform_exit(&ai->inventory);
}

void ai_load(ai_t* ai, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open AI file '%s'\n", filename);
		exit(1);
	}

	fprintf(stderr, "Parsing bot '%s'\n", filename);

	int age = 0;

	char* line = NULL;
	size_t n_line = 0;
	while (getline(&line, &n_line, f) > 0)
	{
		float val = 0;
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

		if (strcmp(key, "Age") == 0)
		{
			age = id;
		}

		if (age != 1)
			continue;

		if (strcmp(key, "Ressource") == 0)
		{
			transform_req(&ai->inventory, id - 1, val, 0);
		}
		else if (strcmp(key, "Objet") == 0)
		{
			transform_req(&ai->inventory, id - 1, val, 1);
		}
		else if (strcmp(key, "Batiment") == 0)
		{
			ai->building = id - 1;
		}
	}

	fclose(f);
}


char ai_gather(character_t* c, int id, float amount)
{
	amount -= c->inventory.materials[id];
	if (amount < 0)
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

	transform_t* tr = NULL;
	for (size_t i = 0; i < u->n_buildings; i++)
	{
		kindOf_building_t* t = &u->buildings[i];
		tr = kindOf_building_available(t, id, 0);
		if (tr == NULL)
			continue;

		if (ai_getreq(c, tr, amount))
			return 1;

		if (ai_build(c, i))
			return 1;

		break;
	}

	if (tr == NULL)
		return 1;

	building_t* b = c->hasBuilding;
	if (c->inBuilding != b)
	{
		c->go_o = &b->o;
		return 1;
	}

	return 1;
}
char ai_make(character_t* c, int id, float amount)
{
	amount -= c->inventory.items[id];
	if (amount < 0)
		return 0;

	transform_t* tr = NULL;
	universe_t* u = c->universe;
	for (size_t i = 0; i < u->n_buildings; i++)
	{
		kindOf_building_t* t = &u->buildings[i];
		tr = kindOf_building_available(t, id, 1);
		if (tr == NULL)
			continue;

		if (ai_getreq(c, tr, amount))
			return 1;

		if (ai_build(c, i))
			return 1;

		break;
	}

	if (tr == NULL)
		return 1;

	building_t* b = c->hasBuilding;
	if (c->inBuilding != b)
	{
		c->go_o = &b->o;
		return 1;
	}

	if (b->work_n != 0)
		return 1;

	int nth = (tr - b->t->items);
	building_work_enqueue(b, nth);
	return 1;
}
char ai_getreq(character_t* c, transform_t* tr, float amount)
{
	for (int i = 0; i < tr->n_req; i++)
	{
		component_t* p = &tr->req[i];
		if ((p->is_item ? ai_make : ai_gather)(c, p->id, p->amount * amount))
			return 1;
	}
	return 0;
}
char ai_build(character_t* c, int id)
{
	kindOf_building_t* t = &c->universe->buildings[id];

	if (c->hasBuilding != NULL && c->hasBuilding->t == t)
		return 0;

	if (ai_getreq(c, &t->build, 1))
		return 1;

	character_buildAuto(c, t);
	return 1;
}
char ai_do(ai_t* ai, character_t* c)
{
	transform_t* tr = &ai->inventory;

	if (ai_getreq(c, tr, 1))
		return 1;

	if (ai->building >= 0 && ai_build(c, ai->building))
		return 1;

	building_t* b = c->hasBuilding;
	if (c->inBuilding != b)
	{
		c->go_o = &b->o;
		return 1;
	}

	return 0;
}

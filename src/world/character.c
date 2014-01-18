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

#include "character.h"

#include <math.h>
#include <string.h>

#include "../util.h"
#include "mine.h"

#define M_PI 3.14159265358979323846

void character_init(character_t* c, universe_t* u, world_t* w)
{
	c->o.t = O_CHARACTER;
	c->o.w = 24;
	c->o.h = 32;

	character_setPosition(c, 0, 0);

	c->go_o = NULL;
	c->dir  = D_SOUTH;
	c->step = 5; // standing still
	c->inWater = 0;

	c->ai        = NULL;

	c->universe = u;
	c->world    = w;

	inventory_init(&c->inventory, u);
	c->hasBuilding = NULL;
	c->inBuilding  = NULL;

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		c->sskills[i] = 1;

	c->mskills = CALLOC(skill_t, u->n_materials);
	for (size_t i = 0; i < u->n_materials; i++)
		c->mskills[i] = 1;

	c->iskills = CALLOC(skill_t, u->n_iskills);
	for (size_t i = 0; i < u->n_iskills; i++)
		c->iskills[i] = 1;

	for (int i = 0; i < N_STATUSES; i++)
		c->statuses[i] = 20;

	c->equipment = CALLOC(int, u->n_slots);
	for (size_t i = 0; i < u->n_slots; i++)
		c->equipment[i] = -1;
}

void character_exit(character_t* c)
{
	free(c->equipment);
	free(c->iskills);
	free(c->mskills);
	inventory_exit(&c->inventory);
}

static float vitality_malus(character_t* c, int i)
{
	float s = c->statuses[i] / 20;
	if (s < 0.10) return 0.450;
	if (s < 0.25) return 0.250;
	if (s < 0.50) return 0.125;
	return 0;
}
float character_vitality(character_t* c)
{
	float ret = 1;
	ret -= vitality_malus(c, ST_HEALTH);
	ret -= vitality_malus(c, ST_STAMINA);
	ret -= vitality_malus(c, ST_MORAL);
	return fmax(ret, 0.1);
}

void character_weary(character_t* c, float f)
{
	c->statuses[ST_STAMINA] = max(c->statuses[ST_STAMINA] - f,   0);
	c->statuses[ST_MORAL]   = max(c->statuses[ST_MORAL]   - f/3, 0);
}

void character_workAt(character_t* c, object_t* o, float duration)
{
	if (o == NULL)
	{
		character_weary(c, -0.01 * duration);
		return;
	}

	if (o->t == O_MINE)
	{
		mine_t* m = (mine_t*) o;
		transform_t* tr = &m->t->harvest;

		if (tr->n_res == 0 || tr->res[0].is_item)
			return;

		int id = tr->res[0].id;

		float skill = c->mskills[id];
		universe_t* u = c->universe;
		for (size_t i = 0; i < u->n_slots; i++)
		{
			int item = c->equipment[i];
			if (item < 0)
				continue;
			kindOf_item_t* t = &u->items[item];
			skill += t->bonus_material[id];
		}

		float work = duration * tr->rate;
		work *= skill;

		transform_apply(&m->t->harvest, &c->inventory, work);

		c->mskills[id] += duration/100;
		character_weary(c, 0.1 * duration);
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;
		kindOf_building_t* t = b->t;

		if (b->build_progress != 1)
		{
			transform_t* tr = &t->build;

			float skill = c->sskills[SK_BUILD];
			universe_t* u = c->universe;
			for (size_t i = 0; i < u->n_slots; i++)
			{
				int item = c->equipment[i];
				if (item < 0)
					continue;
				kindOf_item_t* t = &u->items[item];
				skill += t->bonus_special[SK_BUILD];
			}

			float work = duration * tr->rate;
			work *= skill;

			float rem = 1 - b->build_progress;
			if (work > rem)
				work = rem;

			b->build_progress += work;

			c->sskills[SK_BUILD] += duration/100;
			character_weary(c, 0.3 * duration);
		}
		else if (b->work_n > 0)
		{
			c->inBuilding = b;

			transform_t* tr = &t->items[b->work_list[0]];

			if (tr->n_res == 0 || !tr->res[0].is_item)
				return;

			int id = c->universe->items[tr->res[0].id].skill;

			float skill = c->iskills[id];
			universe_t* u = c->universe;
			for (size_t i = 0; i < u->n_slots; i++)
			{
				int item = c->equipment[i];
				if (item < 0)
					continue;
				kindOf_item_t* t = &u->items[item];
				skill += t->bonus_item[id];
			}

			float work = duration * tr->rate;
			work *= skill;

			float rem  = 1 - b->work_progress;
			if (work > rem)
				work = rem;

			b->work_progress += transform_apply(tr, &c->inventory, work);
			if (b->work_progress >= 1)
			{
				b->work_progress = 0;
				building_work_dequeue(b, 0);
			}

			c->iskills[id] += duration/100;
			character_weary(c, 0.1 * duration);
		}
		else
		{
			c->inBuilding = b;
			transform_t* tr = &t->make;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return;

			int id = tr->res[0].id;

			float skill = c->mskills[id];
			universe_t* u = c->universe;
			for (size_t i = 0; i < u->n_slots; i++)
			{
				int item = c->equipment[i];
				if (item < 0)
					continue;
				kindOf_item_t* t = &u->items[item];
				skill += t->bonus_material[id];
			}

			float work = duration * tr->rate;
			work *= skill;

			transform_apply(tr, &c->inventory, work);

			c->mskills[id] += duration/100;
			character_weary(c, 0.1 * duration);
		}
	}
}

void character_doRound(character_t* c, float duration)
{
	if (c->ai != NULL)
	{
		ai_do(c->ai, c);
	}

	duration *= character_vitality(c);

	float dx;
	float dy;
	if (c->go_o != NULL)
	{
		dx = c->go_o->x;
		dy = c->go_o->y;
	}
	else
	{
		dx = c->go_x;
		dy = c->go_y;
	}
	dx -= c->o.x;
	dy -= c->o.y;

	float remDistance = sqrt(dx*dx + dy*dy);
	if (remDistance == 0)
	{
		c->dir  = D_SOUTH;
		c->step = 5;
		character_workAt(c, c->go_o, duration);
		return;
	}
	c->inBuilding = NULL;

	float dir;
	if (dx > 0)
	{
		dir = atan(dy / dx);
		if (dy < 0)
			dir += 2 * M_PI;
	}
	else if (dx < 0)
	{
		dir = atan(dy / dx) + M_PI;
	}
	else // dx == 0
	{
		dir = M_PI / 2;
		if (dy < 0)
			dir += M_PI;
	}

	c->dir = dir < M_PI * 1/4 ? D_EAST :
	         dir < M_PI * 3/4 ? D_SOUTH :
		 dir < M_PI * 5/4 ? D_WEST :
		 dir < M_PI * 7/4 ? D_NORTH :
		                     D_EAST;

	float distance = 100 * duration;

	c->inWater = 0;
	world_t* w = c->world;
	int t = world_landAt(w, c->o.x, c->o.y);
	if (t == 4) // mountains
	{
		distance /= 3;
	}
	else if (t == 10) // water
	{
		float y = c->o.y - 6;
		if (world_landAt(w, c->o.x-c->o.w/2, y) == t &&
		    world_landAt(w, c->o.x+c->o.w/2, y) == t)
		{
			distance /= 1.5;
			c->inWater = 1;
		}
	}

	if (distance > remDistance)
		distance = remDistance;

	c->o.x += distance * cos(dir);
	c->o.y += distance * sin(dir);

	c->o.x = fmax(c->o.x, -w->o.w/2+12);
	c->o.y = fmax(c->o.y, -w->o.h/2+32);
	c->o.x = fmin(c->o.x,  w->o.w/2-12);
	c->o.y = fmin(c->o.y,  w->o.h/2);

	if (c->step == 5)
		c->step = 0;
	c->step += 0.1 * distance;
	if (c->step >= 4)
		c->step = 0;

	character_weary(c, 0.02*duration);
}

void character_setPosition(character_t* c, float x, float y)
{
	c->o.x = x;
	c->o.y = y;
	c->go_x = x;
	c->go_y = y;
}

void character_goMine(character_t* c, kindOf_mine_t* t)
{
	mine_t* m = world_findMine(c->world, c->o.x, c->o.y, t);
	if (m != NULL)
		c->go_o = &m->o;
}

char character_buildAuto(character_t* c, kindOf_building_t* t)
{
	if (!transform_check(&t->build, &c->inventory))
		return 0;

	for (float radius = 50; radius < 500; radius += 10)
	{
		float x = c->o.x + cfrnd(radius);
		float y = c->o.y + cfrnd(radius);

		if (character_buildAt(c, t, x, y))
			return 1;
	}

	return 0;
}

char character_buildAt(character_t* c, kindOf_building_t* t, float x, float y)
{
	if (!world_canBuild(c->world, x, y, t))
		return 0;

	if (!transform_check(&t->build, &c->inventory))
		return 0;

	if (c->hasBuilding)
	{
		// TODO
	}

	transform_apply(&t->build, &c->inventory, 1);
	c->hasBuilding = world_addBuilding(c->world, t, x, y);
	return 1;
}

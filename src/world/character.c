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
#include <stdio.h>

#include "../mem.h"
#include "../rand.h"
#include "mine.h"

#define M_PI 3.14159265358979323846

void character_init(character_t* c, kindOf_character_t* t, universe_t* u, world_t* w)
{
	c->o.t = O_CHARACTER;
	c->o.w = 24;
	c->o.h = 32;

	c->t = t;
	c->alive = 1;

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

	c->skills = CALLOC(skill_t, u->n_skills);
	for (size_t i = 0; i < u->n_skills; i++)
		c->skills[i] = 20;

	for (int i = 0; i < N_STATUSES; i++)
		c->statuses[i] = 20;

	c->equipment = CALLOC(int, u->n_slots);
	for (size_t i = 0; i < u->n_slots; i++)
		c->equipment[i] = -1;
}

void character_exit(character_t* c)
{
	if (c == NULL)
		return;

	free(c->equipment);
	free(c->skills);
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
	c->statuses[ST_STAMINA] = fmax(c->statuses[ST_STAMINA] - f,   0);
	c->statuses[ST_MORAL]   = fmax(c->statuses[ST_MORAL]   - f/3, 0);
}

float character_getSkill(character_t* c, int skill)
{
	float ret = c->skills[skill];

	// bonuses
	universe_t* u = c->universe;
	for (size_t i = 0; i < u->n_slots; i++)
	{
		int item = c->equipment[i];
		if (item < 0)
			continue;
		kindOf_item_t* t = &u->items[item];
		effect_t* e = &t->effect;
		for (size_t j = 0; j < e->n_skills; j++)
			if (e->skills[j] == skill)
			{
				ret += e->bonuses[j];
				break;
			}
	}

	return ret / 20;
}

void character_train(character_t* c, int skill, float work)
{
	c->skills[skill] += work/50;
	character_weary(c, 0.1 * work);
}

float character_maxOf(character_t* c, kindOf_material_t* m)
{
	return 20 * character_getSkill(c, m->skill);
}

void character_workAt(character_t* c, object_t* o, float duration)
{
	if (o == NULL)
	{
		character_weary(c, -0.01 * duration);
		return;
	}

	universe_t* u = c->universe;
	if (o->t == O_MINE)
	{
		mine_t* m = (mine_t*) o;
		transform_t* tr = &m->t->harvest;

		if (tr->n_res == 0 || tr->res[0].is_item)
			return;

		int id = tr->res[0].id;
		kindOf_material_t* t = &u->materials[id];
		int skill = t->skill;

		float work = duration * character_getSkill(c, skill);
		work *= tr->rate;
		float avail = character_maxOf(c, t) - c->inventory.materials[id];
		work = fmin(work, avail/tr->res[0].amount);
		work = transform_apply(tr, &c->inventory, work);
		character_train(c, skill, work);
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;

		if (b->owner != c)
			return;

		kindOf_building_t* t = b->t;

		if (b->build_progress != 1)
		{
			transform_t* tr = &t->build;

			float work = duration * character_getSkill(c, SK_BUILD);
			work *= tr->rate;
			work = building_build(b, work);
			character_train(c, SK_BUILD, work);
		}
		else if (b->work_n > 0)
		{
			c->inBuilding = b;

			transform_t* tr = &t->items[b->work_list[0]];

			if (tr->n_res == 0 || !tr->res[0].is_item)
				return;

			int id = tr->res[0].id;
			kindOf_item_t* it = &u->items[id];
			int skill = it->skill;

			float work = duration * character_getSkill(c, skill);
			work *= tr->rate;
			work = fmin(work, 1 - b->work_progress);
			work = transform_apply(tr, &c->inventory, work);
			character_train(c, skill, work);

			b->work_progress += work;
			if (b->work_progress >= 1)
			{
				b->work_progress = 0;
				building_work_dequeue(b, 0);
			}
		}
		else
		{
			c->inBuilding = b;
			transform_t* tr = &t->make;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			int skill = t->skill;

			float work = duration * character_getSkill(c, skill);
			work *= tr->rate;
			float avail = character_maxOf(c, t) - c->inventory.materials[id];
			work = fmin(work, avail/tr->res[0].amount);
			work = transform_apply(tr, &c->inventory, work);
			character_train(c, skill, work);
		}
	}
}

char character_eat(character_t* c, int material)
{
	kindOf_material_t* m = &c->universe->materials[material];
	if (!m->edible)
		return 0;

	if (c->inventory.materials[material] < 1)
		return 0;

	c->inventory.materials[material] -= 1;
	for (int i = 0; i < N_STATUSES; i++)
		c->statuses[i] += m->eatBonus[i];
	return 1;
}

void character_eatFor(character_t* c, int status)
{
	universe_t* u = c->universe;
	for (size_t i = 0; i < u->n_materials; i++)
	{
		kindOf_material_t* m = &u->materials[i];
		if (m->eatBonus[status] > 0 && character_eat(c, i))
			return;
	}
}

void character_move(character_t* c, float duration, float dx, float dy)
{
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

	float distance = duration * character_getSkill(c, SK_WALK);
	distance *= 100;

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

	float remDistance = sqrt(dx*dx + dy*dy);
	distance = fmin(distance, remDistance);

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

	character_train(c, SK_WALK, distance / 100);
}

void character_doRound(character_t* c, float duration)
{
	if (!c->alive)
		return;

	if (c->ai != NULL)
	{
		ai_do(c->ai, c);
	}

	duration *= character_vitality(c);

	float go_x = c->go_x;
	float go_y = c->go_y;
	if (c->go_o != NULL)
	{
		go_x = c->go_o->x;
		go_y = c->go_o->y;
		c->go_x = c->o.x;
		c->go_y = c->o.y;
	}
	float dx = go_x - c->o.x;
	float dy = go_y - c->o.y;

	float remDistance = sqrt(dx*dx + dy*dy);
	if (c->go_o != NULL && remDistance < 20)
	{
		if (c->go_o->t == O_CHARACTER)
		{
			character_t* t = (character_t*) c->go_o;
			if (t != c)
			{
				float work = duration * character_getSkill(c, SK_ATTACK);
				work = character_attacked(t, work, c);
				character_train(c, SK_ATTACK, work);
				return;
			}
		}
		else if (c->go_o->t == O_BUILDING)
		{
			building_t* b = (building_t*) c->go_o;
			if (b->owner != c)
			{
				float work = duration * character_getSkill(c, SK_DESTROY);
				work = building_attacked(b, work, c);
				character_train(c, SK_DESTROY, work);
				return;
			}
		}
	}

	if (remDistance == 0)
	{
		c->dir  = D_SOUTH;
		c->step = 5;
		character_workAt(c, c->go_o, duration);
	}
	else
		character_move(c, duration, dx, dy);
}

void character_setPosition(character_t* c, float x, float y)
{
	c->o.x = x;
	c->o.y = y;
	c->go_x = x;
	c->go_y = y;
}

float character_attacked(character_t* c, float work, character_t* a)
{
	work = fmin(work, c->statuses[ST_HEALTH]);
	c->statuses[ST_HEALTH] -= work;
	if (c->statuses[ST_HEALTH] <= 0)
	{
		c->alive = 0;
		a->go_o = NULL;
	}
	return work;
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

	character_delHome(c);

	for (float radius = 50; radius < c->world->o.w; radius *= 1.5)
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

	character_delHome(c);

	transform_apply(&t->build, &c->inventory, 1);
	c->hasBuilding = world_addBuilding(c->world, x, y, t, c);
	return 1;
}

char character_delHome(character_t* c)
{
	if (c->hasBuilding == NULL)
		return 0;

	// TODO: there might be other pointers...
	if (c->go_o == &c->hasBuilding->o)
		c->go_o = NULL;
	if (c->inBuilding == c->hasBuilding)
		c->inBuilding = NULL;

	world_delBuilding(c->world, c->hasBuilding);
	c->hasBuilding = NULL;
	return 1;
}

size_t character_currentAction(character_t* c, char* buffer, size_t n)
{
	size_t cur = 0;

	object_t* o = c->go_o;
	universe_t* u = c->universe;
	char moving = c->go_x != c->o.x || c->go_y != c->o.y;
	if (o == NULL)
	{
		if (moving)
			cur += snprintf(buffer+cur, n-cur, "Marcher");
		else
			cur += snprintf(buffer+cur, n-cur, "Attendre");
	}
	else if (o->t == O_MINE)
	{
		mine_t* m = (mine_t*) o;

		if (moving)
		{
			cur += snprintf(buffer+cur, n-cur, "Aller à %s", m->t->name);
		}
		else
		{
			transform_t* tr = &m->t->harvest;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return cur;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			int skill = t->skill;

			float amount = c->inventory.materials[id];
			float max = character_maxOf(c, t);
			cur += snprintf(buffer+cur, n-cur, "%s (%.0f / %0.f)", u->skills[skill].name, amount, max);
		}
	}
	else if (o->t == O_CHARACTER)
	{
		character_t* t = (character_t*) o;
		const char* name = t->ai == NULL ? "ennemi" : t->ai->name;
		if (t != c)
			cur += snprintf(buffer+cur, n-cur, "Attaquer %s\n", name);
		else
			cur += snprintf(buffer+cur, n-cur, "S'admirer");
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;
		kindOf_building_t* t = b->t;
		const char* name = t->name;
		if (b->owner != c)
		{
			cur += snprintf(buffer+cur, n-cur, "Attaquer %s\n", name);
		}
		else if (moving)
		{
			cur += snprintf(buffer+cur, n-cur, "Se diriger vers %s\n", name);
		}
		else if (b->build_progress != 1)
		{
			cur += snprintf(buffer+cur, n-cur, "%s", u->skills[SK_BUILD].name);
		}
		else if (b->work_n > 0)
		{
			transform_t* tr = &t->items[b->work_list[0]];

			if (tr->n_res == 0 || !tr->res[0].is_item)
				return cur;

			int id = tr->res[0].id;
			kindOf_item_t* it = &u->items[id];

			cur += snprintf(buffer, 1024, "%s (%i%%)", it->name, (int) floor(100*b->work_progress));
		}
		else if (t->make.n_res != 0)
		{
			transform_t* tr = &t->make;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return cur;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			int skill = t->skill;

			float amount = c->inventory.materials[id];
			float max = character_maxOf(c, t);
			cur += snprintf(buffer+cur, n-cur, "%s (%.0f / %0.f)", u->skills[skill].name, amount, max);
		}
	}

	return cur;
}

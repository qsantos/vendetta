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

#include "../math.h"

char character_doAttack(character_t* c, object_t* o)
{
	if (o == NULL)
		return 0;


	if (o->t == O_CHARACTER)
	{
		character_t* t = (character_t*) o;

		// cannot attack self
		if (c == t)
			return 0;

		building_t* b = character_get_inBuilding(t);
		if (b != NULL || !t->alive)
		{
			c->go_o = -1;
			return 0;
		}
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;

		// cannot attack a building from within
		if (b->o.uuid == c->inBuilding)
			return 0;

		// cannot attack own building
		if (b->owner == c->o.uuid)
			return 0;
	}
	else
		return 0;

	universe_t* u = c->w->universe;

	// TODO
	int id = c->equipment[0];
	kindOf_item_t* it = id < 0 ? NULL : &u->items[id];

	float range = 20 + (it == NULL ? 0 : it->range);
	float d = object_distance(&c->o, o->x, o->y);
	if (d >= range && !(o->t == O_BUILDING && object_overlaps(&c->o, o)))
		return 0;

	if (c->attackDelay != 0)
		return 1;

	if (it != NULL && transform_check(&it->cost, &c->inventory) == 0)
		return 1;

	float reqMana = it == NULL ? 0 : it->reqMana;
	if (c->statuses[ST_MANA] < reqMana)
		return 1;

	if (c->statuses[ST_ATTACK] < 7)
		return 1;

	c->statuses[ST_ATTACK] -= 7;
	c->statuses[ST_MANA]   -= reqMana;

	if (it != NULL)
		transform_apply(&it->cost, &c->inventory, 1);

	c->attackDelay = it == NULL ? 1 : it->reloadDelay;

	if (it != NULL && it->projectile >= 0)
	{
		float work = 5*character_getSkill(c, SK_ATTACK);
		character_train(c, SK_ATTACK, work);

		kindOf_projectile_t* pt = &u->projectiles[it->projectile];
		projectile_t* p = projectile_new(&c->w->objects, -1);
		projectile_init(p, c->w, pt, c->o.x, c->o.y, work, o->x, o->y);

		return 1;
	}
	else if (o->t == O_CHARACTER)
	{
		character_t* t = (character_t*) o;

		float work = 5*character_getSkill(c, SK_ATTACK);
		work = character_attacked(t, work);
		character_train(c, SK_ATTACK, work);
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;

		float work = character_getSkill(c, SK_DESTROY);
		work = building_attacked(b, work);
		character_train(c, SK_DESTROY, work);
	}

	int evtId = it == NULL ? 0 : it->event;
	if (evtId < 0)
		evtId = 0;
	kindOf_event_t* e = &u->events[evtId];
	evtList_push(&c->w->events, e, o->x, o->y - o->h/2);

	return 1;
}

void character_doWork(character_t* c, object_t* o, float duration)
{
	if (o == NULL)
	{
		character_weary(c, -0.01 * duration);
		return;
	}

	universe_t* u = c->w->universe;
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
		float avail = character_maxOfMaterial(c, t) - c->inventory.materials[id];
		work = fmin(work, avail/tr->res[0].amount);
		work = transform_apply(tr, &c->inventory, work);
		character_train(c, skill, work);
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;

		if (b->owner != c->o.uuid)
		{
			if (b->build_progress == 1)
				c->inBuilding = b->o.uuid;
			return;
		}

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
			c->inBuilding = b->o.uuid;

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
			c->inBuilding = b->o.uuid;
			transform_t* tr = &t->make;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			int skill = t->skill;

			// how much work can be done with the given time and resources
			float work = duration * character_getSkill(c, skill);
			work *= tr->rate;
			work = transform_apply(tr, &c->inventory, work);
			character_train(c, skill, work);

			// transfers exceeding resources to building
			float max = character_maxOfMaterial(c, t);
			float cur = c->inventory.materials[id];
			if (cur > max)
			{
				inventory_mov(&b->inventory, MATERIAL, id, cur-max, &c->inventory);
				building_update(b);
			}
		}
	}
}

void character_doMove(character_t* c, float duration, float dx, float dy)
{
	c->inBuilding = -1;

	float dir = atan2f(dy, dx);

	c->dir = dir <-M_PI * 3/4 ? D_WEST :
	         dir <-M_PI * 1/4 ? D_NORTH :
	         dir < M_PI * 1/4 ? D_EAST :
	         dir < M_PI * 3/4 ? D_SOUTH :
	                            D_WEST;

	float distance = duration * character_getSkill(c, SK_WALK);
	distance *= 100;

	c->inWater = 0;
	world_t* w = c->w;
	int l = world_getLandXY(w, c->o.x, c->o.y)/16;
	if (l == 4) // mountains
	{
		distance /= 3;
	}
	else if (l == 10) // water
	{
		float y = c->o.y - 6;
		if (world_getLandXY(w, c->o.x-c->o.w/2, y)/16 == l &&
		    world_getLandXY(w, c->o.x+c->o.w/2, y)/16 == l)
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

	c->attackDelay = fmax(c->attackDelay - duration, 0);

	duration *= character_vitality(c);

	character_addStatus(c, ST_ATTACK,  7*duration);
	character_addStatus(c, ST_DEFENSE, 3*duration);

	float go_x = c->go_x;
	float go_y = c->go_y;
	object_t* o = pool_get(&c->w->objects, c->go_o);
	if (o != NULL)
	{
		// stay still if target is removed
		c->go_x = c->o.x;
		c->go_y = c->o.y;

		go_x = o->x;
		go_y = o->y;

		// door offset
		if (o->t == O_BUILDING)
		{
			building_t* b = (building_t*) o;
			go_x += b->t->door_dx;
			go_y += b->t->door_dy;
		}
	}
	float dx = go_x - c->o.x;
	float dy = go_y - c->o.y;

	float remDistance = sqrt(dx*dx + dy*dy);
	if (c->attack && character_doAttack(c, o))
		return;

	if (remDistance == 0)
	{
		c->dir  = D_SOUTH;
		c->step = 5;
		character_doWork(c, o, duration);
	}
	else
		character_doMove(c, duration, dx, dy);
}

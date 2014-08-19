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

#include "draw.h"

#include <stdlib.h>
#include <math.h>

#include "../widgets.h"

static void draw_object(graphics_t* g, assets_t* a, object_t* o, sfSprite* sprite)
{
	(void) a;

	sfVector2f pos = {o->x - o->w/2, o->y - o->h};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_event(graphics_t* g, assets_t* a, character_t* player, event_t* e)
{
	(void) player;

	kindOf_event_t* t = e->t;

	if (t->sprite < 0)
		return;

	int step = floor(e->p * t->steps);
	if (step >= t->steps)
		return;

	sfSprite* sprite = a->sprites[t->sprite];

	sfIntRect rect = {0, 0, t->width, t->height};
	rect.left = t->width * step;
	sfSprite_setTextureRect(sprite, rect);

	sfVector2f pos = {e->x - t->width/2, e->y - t->height/2};
	sfSprite_setPosition(sprite, pos);

	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_projectile(graphics_t* g, assets_t* a, character_t* player, projectile_t* p)
{
	(void) player;

	if (p->t->sprite < 0)
		return;

	sfSprite* sprite = a->sprites[p->t->sprite];

	int step = floor(p->step);
	if (step >= 3)
		step = 1;

	float w = p->t->width;
	float h = p->t->height;
	sfIntRect  rect = {w*step, h*p->dir, w, h};
	sfVector2f pos  = {p->o.x - p->o.w/2, p->o.y - p->o.h};

	sfSprite_setTextureRect(sprite, rect);
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_character(graphics_t* g, assets_t* a, character_t* player, character_t* c)
{
	if (c == NULL)
		return;

	building_t* b = character_get_inBuilding(c);
	if (b != NULL)
		return;

	int step = floor(c->step);
	if (step >= 3)
		step = 1;

	sfIntRect  rect = {24*step, 32*c->dir, 24, 32};
	sfVector2f pos  = {c->o.x - c->o.w/2, c->o.y - c->o.h};

	if (c->inWater)
	{
		pos.y += 12;
		rect.height -= 12;
	}
	else if (c == player)
	{
		static sfCircleShape* circle = NULL;
		static const float radius = 10;
		static const float scale = 0.5;
		if (circle == NULL)
		{
			circle = sfCircleShape_create();
			sfCircleShape_setRadius(circle, radius);
			sfCircleShape_setScale(circle, (sfVector2f){1,scale});
			sfCircleShape_setFillColor(circle, sfTransparent);
			sfCircleShape_setOutlineColor(circle, sfWhite);
			sfCircleShape_setOutlineThickness(circle, 2);
		}
		sfCircleShape_setPosition(circle, (sfVector2f){c->o.x-radius, c->o.y-radius*scale-2});
		sfRenderWindow_drawCircleShape(g->render, circle, NULL);
	}

	static sfSprite* defaultSprite = NULL;
	if (defaultSprite == NULL)
		defaultSprite = assets_sprite(a, "data/characters/default.png");

	sfSprite*sprite = c->t == NULL ? defaultSprite : a->sprites[c->t->sprite];

	sfColor color = c->alive ? sfWhite : (sfColor){255,255,255,127};
	sfSprite_setColor(sprite, color);

	sfSprite_setTextureRect(sprite, rect);
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);

	if (!c->alive)
		return;

	float max1 = character_maxOfStatus(c, ST_HEALTH);
	float max2 = character_maxOfStatus(c, ST_ATTACK);
	float max3 = character_maxOfStatus(c, ST_DEFENSE);
	float p1 = c->statuses[ST_HEALTH]  / max1;
	float p2 = c->statuses[ST_ATTACK]  / max2;
	float p3 = c->statuses[ST_DEFENSE] / max3;
	char draw2 = p2 != 1 || p3 != 1;
	char draw1 = draw2 || p1 != 1;
	if (draw1)
	{
		draw_progressbar(g, c->o.x - c->o.w/2, c->o.y+6, c->o.w, 5, p1, 0);
	}
	if (draw2)
	{
		draw_progressbar(g, c->o.x - c->o.w/2, c->o.y+10, c->o.w, 5, p2, -3);
		draw_progressbar(g, c->o.x - c->o.w/2, c->o.y+14, c->o.w, 5, p3, -4);
	}
}

void draw_mine(graphics_t* g, assets_t* a, character_t* player, mine_t* m)
{
	(void) player;

	if (m == NULL)
		return;

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
		sprite = assets_sprite(a, "data/mines.png");

	int t = m->t->id;
	sfIntRect rect = {32*t, 32*0, 32, 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, a, &m->o, sprite);
}

void draw_building(graphics_t* g, assets_t* a, character_t* player, building_t* b)
{
	if (b == NULL)
		return;

	int step;
	int n = b->t->n_sprites;
	float p = b->build_progress;

	if (n <= 1)
		step = 0;
	else if (b->open)
		step = n-1;
	else
		step = floor(p * (n-2));

	sfSprite* sprite = a->sprites[b->t->sprite];
	sfIntRect rect = {0, b->o.h*step, b->o.w, b->o.h};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, a, &b->o, sprite);

	if (player->hasBuilding == b->o.uuid && player->inBuilding == b->o.uuid && p == 1)
	{
		universe_t* u = player->w->universe;
		float p;
		if (b->work_n > 0)
		{
			p = b->work_progress;
		}
		else
		{
			transform_t* tr = &b->t->make;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			p = player->inventory.materials[id] / character_maxOfMaterial(player, t);
		}
		draw_progressbar(g, b->o.x - b->o.w/2, b->o.y-b->o.h-6, b->o.w, 5, p, -1);
	}

	p = b->life / 20;
	if (p < 0.99)
		draw_progressbar(g, b->o.x - b->o.w/2, b->o.y+1, b->o.w, 5, p, 0);
}

void draw_chunkLands(graphics_t* g, assets_t* a, character_t* player, chunk_t* c, int step)
{
	(void) player;

	static sfRenderStates states = {sfBlendAlpha, {{1,0,0,0,1,0,0,0,1}}, NULL, NULL};
	if (states.texture == NULL)
		states.texture = assets_loadImage(a, "data/lands.png");

	sfVertexArray* array = c->array;
	if (step != c->water_step)
	{
		c->water_step = step;
		chunk_updwtr(c);
	}

	sfRenderWindow_drawVertexArray(g->render, array, &states);
}

void draw_chunkMines(graphics_t* g, assets_t* a, character_t* player, chunk_t* c)
{
	for (ssize_t i = c->n_mines-1; i >= 0; i--)
		draw_mine(g, a, player, c->mines[i]);
}

void draw_chunks(graphics_t* g, assets_t* a, character_t* player, world_t* w, int step)
{
	sfVector2f x = sfView_getCenter(g->world_view);
	sfVector2f s = sfView_getSize(g->world_view);

	// quickfix: force chunks with jutting potential mines
	s.x += 64;
	s.y += 64;

	object_t o = {O_NONE, 0, x.x, x.y+s.y/2, s.x, s.y};
	for (size_t i = 0; i < w->n_chunks; i++)
	{
		chunk_t* c = &w->chunks[i];
		if (object_overlaps(&c->o, &o))
			draw_chunkLands(g, a, player, c, step);
	}

	for (size_t i = 0; i < w->n_chunks; i++)
	{
		chunk_t* c = &w->chunks[i];
		if (object_overlaps(&c->o, &o))
			draw_chunkMines(g, a, player, c);
	}
}

void draw_world(graphics_t* g, assets_t* a, character_t* player, world_t* w, int step)
{
	draw_chunks(g, a, player, w, step);

	pool_t* p = &w->objects;
	for (ssize_t i = p->n_objects-1; i >= 0; i--)
	{
		building_t* b = building_get(p, i);
		if (b != NULL)
			draw_building(g, a, player, b);
	}

	for (ssize_t i = p->n_objects-1; i >= 0; i--)
	{
		character_t* c = character_get(p, i);
		if (c != NULL)
			draw_character(g, a, player, c);
	}

	for (ssize_t i = p->n_objects-1; i >= 0; i--)
	{
		projectile_t* q = projectile_get(p, i);
		if (q != NULL)
			draw_projectile(g, a, player, q);
	}

	for (ssize_t i = w->events.n-1; i >= 0; i--)
		draw_event(g, a, player, &w->events.d[i]);
}

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

#include "build.h"

#include <stdlib.h>
#include <stdio.h>
#include <math.h>

#include "../mem.h"

#define M_PI 3.14159265358979323846

#define PANEL_N_COLS 3

void ov_build_init(ov_build_t* o)
{
	o->selected = NULL;

	o->retracted = 1;

	o->active = 0;
	o->x = 0;
	o->y = 0;
	o->radius = 50;
	o->count = 8;
	o->list = CALLOC(int, o->count);
	o->list[0] =  0;
	o->list[1] =  2;
	o->list[2] =  6;
	o->list[3] =  9;
	o->list[4] = 15;
	o->list[5] = 18;
	o->list[6] = 19;
	o->list[7] = 20;

}

void ov_build_exit(ov_build_t* o)
{
	free(o->list);
}

static int draw_buildButton(ov_build_t* o, game_t* g, kindOf_building_t* b, float x, float y, char do_draw)
{
	(void) o;

	sfVector2f mouse;
	if (!do_draw)
		mouse = overlay_mouse(g);

	char caught = 0;

	sfSprite* sprite = g->g->sprites[b->button_sprite];

	static const int max_cols = 1024 / 28;
	int idx = b->button_index;
	int ok = transform_check(&b->build, &g->player->inventory);

	int col = idx % max_cols;
	int row = idx / max_cols;

	sfColor enabled  = {255,255,255,255};
	sfColor disabled = { 95, 95, 95,255};
	sfIntRect rect = {28*col, 28*row, 28, 28};

	sfSprite_setTextureRect(sprite, rect);
	sfSprite_setPosition(sprite, (sfVector2f){x,y});
	sfSprite_setColor(sprite, ok ? enabled : disabled);
	if (do_draw)
		sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
	else
		caught |= sfSprite_contains(sprite, mouse);

	return caught;
}

static int draw_buildPanel(ov_build_t* o, game_t* g, char do_draw)
{
	float x = 0;
	float y = 100;
	char caught = 0;

	if (do_draw)
	{
		o->retracted  = draw_buildPanel(o, g, 0) < 0;
		o->retracted &= draw_buildPanel(o, g, 0) < 0;
	}

	for (size_t i = 0; i < g->u->n_buildings; i++)
	{
		kindOf_building_t* b = &g->u->buildings[i];
		if (b->button_sprite < 0)
			continue;

		// building button
		caught |= draw_buildButton(o, g, b, o->retracted ? x - 80 : x, y, do_draw);

		x += 28;
		if (x >= 28*PANEL_N_COLS)
		{
			x = 0;
			y += 28;
		}

		if (caught)
			return i;
	}

	return -1;
}

static int draw_buildWheel(ov_build_t* o, game_t* g, char do_draw)
{
	if (!o->active)
		return -1;

	// circle
	float radius = o->radius;
	static sfCircleShape* shape = NULL;
	if (shape == NULL)
	{
		shape = sfCircleShape_create();
		sfCircleShape_setFillColor(shape, sfTransparent);
		sfCircleShape_setOutlineThickness(shape, 5);
		sfCircleShape_setOutlineColor(shape, (sfColor){255,255,255,127});
		sfCircleShape_setRadius(shape, radius);
		sfCircleShape_setPointCount(shape, 100);
	}
	sfVector2f pos = {o->x - radius, o->y - radius};
	sfCircleShape_setPosition(shape, pos);
	if (do_draw)
		sfRenderWindow_drawCircleShape(g->g->render, shape, NULL);

	char caught = 0;

	int n = o->count;
	float a = 2*M_PI/n;
	float cur = 0;
	for (int i = 0; i < n; i++)
	{
		float x = o->x + radius * cos(cur) - 14;
		float y = o->y + radius * sin(cur) - 14;

		int id = o->list[i];
		kindOf_building_t* b = &g->u->buildings[id];

		caught |= draw_buildButton(o, g, b, x, y, do_draw);

		cur += a;

		if (caught)
			return id;
	}

	return -1;
}

int ov_build_draw(ov_build_t* o, game_t* g, char do_draw)
{
	int r;
	r = draw_buildPanel(o, g, do_draw); if (r >= 0) return r;
	r = draw_buildWheel(o, g, do_draw); if (r >= 0) return r;
	return -1;
}

void ov_build_tooltip(char* buffer, size_t n, game_t* g, kindOf_building_t* b)
{
	size_t cur = 0;
	cur += snprintf(buffer+cur, n-cur, "%s", b->name);

	// required materials
	if (b->build.n_req != 0)
		cur += snprintf(buffer+cur, n-cur, "\n\nRequiert:");
	for (int i = 0; i < b->build.n_req; i++)
	{
		component_t* c = &b->build.req[i];
		if (c->amount == 0)
			continue;
		kindOf_material_t* m = &g->u->materials[c->id];
		cur += snprintf(buffer+cur, n-cur, "\n- %.1f de %s", c->amount, m->name);
	}

	// available material
	if (b->make.n_res != 0)
	{
		int id = b->make.res[0].id;
		kindOf_material_t* m = &g->u->materials[id];
		cur += snprintf(buffer+cur, n-cur, "\n\nPermet de produire : %s", m->name);
	}

	// list items
	if (b->n_items != 0)
		cur += snprintf(buffer+cur, n-cur, "\n\nPermet de fabriquer :");
	for (size_t i = 0; i < b->n_items; i++)
	{
		int id = b->items[i].res[0].id;
		kindOf_item_t* it = &g->u->items[id];
		cur += snprintf(buffer+cur, n-cur, "\n- %s", it->name);
	}
}

int ov_build_cursor(ov_build_t* o, game_t* g)
{
	char caught = 0;

	// tooltip
	int i = ov_build_draw(o, g, 0);
	if (i >= 0)
	{
		kindOf_building_t* b = &g->u->buildings[i];
		char buffer[1024];
		ov_build_tooltip(buffer, 1024, g, b);
		graphics_drawTooltip(g->g, buffer);
		caught = 1;
	}

	// draw selected building

	kindOf_building_t* b = o->selected;
	if (b == NULL)
		return caught ? 4 : -1;

	sfRenderWindow_setView(g->g->render, g->g->world_view);

	sfVector2i imouse = sfMouse_getPositionRenderWindow(g->g->render);
	sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, imouse, g->g->world_view);

	int ok = world_canBuild(g->w, pos.x, pos.y + b->height/2, b);

	// building sprite
	sfSprite* sprite = g->g->sprites[b->sprite];
	sfIntRect rect2 = {0, b->height*(b->n_sprites-1), b->width, b->height};
	sfSprite_setTextureRect(sprite, rect2);
	pos.x -= b->width/2;
	pos.y -= b->height/2;
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->g->render, sprite, NULL);

	// info frame
	static sfRectangleShape* shape = NULL;
	if (shape == NULL)
	{
		shape = sfRectangleShape_create();
		sfRectangleShape_setFillColor(shape, sfTransparent);
		sfRectangleShape_setOutlineThickness(shape, 1);
	}
	sfColor outline = {255*(1-ok), 255*ok, 0, 255};
	sfRectangleShape_setOutlineColor(shape, outline);
	sfVector2f size = {b->width, b->height};
	sfRectangleShape_setSize(shape, size);
	sfRectangleShape_setPosition(shape, pos);
	sfRenderWindow_drawRectangleShape(g->g->render, shape, NULL);

	sfRenderWindow_setView(g->g->render, g->g->overlay_view);
	return 4;
}

char ov_build_catch(ov_build_t* o, game_t* g, int t)
{
	sfVector2f mouse = overlay_mouse(g);

	if (t == sfMouseRight)
	{
		if (o->selected != NULL)
			o->selected = NULL;
		else
		{
			o->active ^= 1;
			o->x = mouse.x;
			o->y = mouse.y;
		}
		return 1;
	}

	if (t != sfMouseLeft)
		return 0;

	int i = ov_build_draw(o, g, 0);
	if (i >= 0)
	{
		o->selected = &g->u->buildings[i];
		o->active = 0;
		return 1;
	}

	if (o->active)
	{
		o->active = 0;
		return 1;
	}

	kindOf_building_t* b = o->selected;
	if (b != NULL)
	{
		sfVector2i imouse = {mouse.x, mouse.y};
		sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, imouse, g->g->world_view);
		pos.y += b->height / 2;

		if (character_buildAt(g->player, b, pos.x, pos.y))
			o->selected = NULL;
		return 1;
	}

	return 0;
}

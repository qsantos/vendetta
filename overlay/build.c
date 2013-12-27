#include "build.h"

#include <math.h>

#include "../util.h"

#define PANEL_N_COLS 3

void ov_build_init(ov_build_t* o)
{
	o->selected = NULL;

	o->active = 0;
	o->x = 0;
	o->y = 0;
	o->radius = 50;
	o->count = 8;
	o->list = CALLOC(int, o->count);
	o->list[0] =  2;
	o->list[1] =  6;
	o->list[2] =  7;
	o->list[3] =  9;
	o->list[4] = 17;
	o->list[5] = 20;
	o->list[6] = 21;
	o->list[7] = 22;

}

void ov_build_exit(ov_build_t* o)
{
	free(o->list);
}

void draw_buildButton(game_t* g, kindOf_building_t* b, float x, float y)
{
	sfSprite* sprite = g->g->sprites[b->button_sprite];

	int ok = transform_check(&b->build, &g->player->inventory);
	sfIntRect rect = {28*b->button_index, 28*ok, 28, 28};
	sfSprite_setTextureRect(sprite, rect);

	sfSprite_setPosition(sprite, (sfVector2f){x,y});

	sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
}

void draw_buildPanel(game_t* g)
{
	float x = 0;
	float y = 0;
	for (int i = 0; i < g->u->n_buildings; i++)
	{
		kindOf_building_t* b = &g->u->buildings[i];
		if (b->button_sprite < 0)
			continue;

		draw_buildButton(g, b, x, y);

		x += 28;
		if (x >= 28*PANEL_N_COLS)
		{
			x = 0;
			y += 28;
		}
	}
}

void ov_build_draw(ov_build_t* o, game_t* g)
{
	draw_buildPanel(g);

	if (!o->active)
		return;

	float radius = o->radius;
	static sfCircleShape* shape = NULL;
	if (shape == NULL)
	{
		shape = sfCircleShape_create();
		sfCircleShape_setFillColor   (shape, (sfColor){0,0,0,0});
		sfCircleShape_setOutlineThickness(shape, 5);
		sfCircleShape_setOutlineColor(shape, (sfColor){255,255,255,127});
		sfCircleShape_setRadius(shape, radius);
		sfCircleShape_setPointCount(shape, 100);
	}

	sfVector2f pos = {o->x - radius, o->y - radius};
	sfCircleShape_setPosition(shape, pos);
	sfRenderWindow_drawCircleShape(g->g->render, shape, NULL);

	int n = o->count;
	float a = 2*M_PI/n;
	float cur = 0;
	for (int i = 0; i < n; i++)
	{
		float x = o->x + radius * cos(cur) - 14;
		float y = o->y + radius * sin(cur) - 14;

		int id = o->list[i];
		kindOf_building_t* b = &g->u->buildings[id];
		draw_buildButton(g, b, x, y);

		cur += a;
	}
}

char ov_build_cursor(ov_build_t* o, game_t* g, float x, float y)
{
	kindOf_building_t* b = o->selected;
	if (b == NULL)
		return 0;

	sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, (sfVector2i){x,y}, g->g->world_view);
	pos.y += b->height/2;
	int ok = world_canBuild(g->w, g->player, b, pos.x, pos.y);

	sfSprite* sprite = g->g->sprites[b->sprite];
	sfIntRect rect = {0, b->height*(b->n_sprites-1), b->width, b->height};
	sfSprite_setTextureRect(sprite, rect);

	pos.x = x - b->width/2;
	pos.y = y - b->height/2;
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->g->render, sprite, NULL);

	sfRectangleShape* shape = NULL;
	if (shape == NULL)
	{
		shape = sfRectangleShape_create();
		sfColor fill = {0, 0, 0, 0};
		sfRectangleShape_setFillColor(shape, fill);
		sfColor outline = {255*(1-ok), 255*ok, 0, 255};
		sfRectangleShape_setOutlineColor(shape, outline);
		sfRectangleShape_setOutlineThickness(shape, 1);
	}

	sfVector2f size = {b->width, b->height};
	sfRectangleShape_setSize(shape, size);
	sfRectangleShape_setPosition(shape, pos);
	sfRenderWindow_drawRectangleShape(g->g->render, shape, NULL);

	return 1;
}

char ov_build_catch(ov_build_t* o, game_t* g, float x, float y, float t)
{
	if (t == sfMouseLeft)
	{
		sfFloatRect rect = {0, 0, 28, 28};
		for (int i = 0; i < g->u->n_buildings; i++)
		{
			kindOf_building_t* b = &g->u->buildings[i];

			if (b->button_sprite < 0)
				continue;

			if (sfFloatRect_contains(&rect, x, y))
			{
				if (transform_check(&b->build, &g->player->inventory))
					o->selected = b;
				return 1;
			}

			rect.left += 28;
			if (rect.left >= 28*PANEL_N_COLS)
			{
				rect.left = 0;
				rect.top += 28;
			}
		}

		if (o->active)
		{
			sfFloatRect rect = {0, 0, 28, 28};

			float radius = o->radius;
			int n = o->count;
			float a = 2*M_PI/n;
			float cur = 0;
			for (int i = 0; i < n; i++)
			{
				rect.left = o->x + radius * cos(cur) - 14;
				rect.top  = o->y + radius * sin(cur) - 14;

				if (sfFloatRect_contains(&rect, x, y))
				{
					int id = o->list[i];
					kindOf_building_t* b = &g->u->buildings[id];
					if (transform_check(&b->build, &g->player->inventory))
						o->selected = b;
					break;
				}

				cur += a;
			}
			o->active = 0;
			return 1;
		}
	}

	kindOf_building_t* b = o->selected;
	if (t == sfMouseRight)
	{
		if (b != NULL)
		{
			o->selected = NULL;
			return 1;
		}
		else
		{
			o->x = x;
			o->y = y;
			o->active ^= 1;
			return 1;
		}
	}
	else if (b != NULL)
	{
		sfVector2i cursor = {x, y};
		sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, cursor, g->g->world_view);
		pos.y += b->height / 2;

		if (!world_canBuild(g->w, g->player, b, pos.x, pos.y))
			return 1;

		transform_apply(&b->build, &g->player->inventory, 1);
		world_addBuilding(g->w, b, pos.x, pos.y);

		o->selected = NULL;
		return 1;
	}

	return 0;
}

#include "draw.h"

#include <stdlib.h>

void draw_object(graphics_t* g, object_t* o, sfSprite* sprite)
{
	sfVector2f pos = {o->x - o->w/2, o->y - o->h};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_character(graphics_t* g, character_t* c)
{
	if (c->inBuilding != NULL)
		return;

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "character.png");
		sprite = g->sprites[id];
	}

	sfIntRect rect = {24*1, 32*c->dir, 24, 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &c->o, sprite);
}

void draw_mine(graphics_t* g, mine_t* m)
{
	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "mines.png");
		sprite = g->sprites[id];
	}

	int t = m->t->id;
	sfIntRect rect = {32*t, 32*0, 32, 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &m->o, sprite);
}

#define BORDER_SIZE 1
static void draw_progressBar(graphics_t* g, float x, float y, float w, float h, float p)
{
	static sfRectangleShape* frame    = NULL;
	static sfRectangleShape* progress = NULL;
	if (frame == NULL)
	{
		frame = sfRectangleShape_create();
		sfColor fill = {0, 0, 0, 0};
		sfRectangleShape_setFillColor(frame, fill);
		sfColor outline = {255, 255, 255, 255};
		sfRectangleShape_setOutlineThickness(frame, BORDER_SIZE);
		sfRectangleShape_setOutlineColor(frame, outline);

		progress = sfRectangleShape_create();
		sfColor inner = {0, 255, 0, 255};
		sfRectangleShape_setFillColor(progress, inner);
	}
	sfVector2f pos = {x+BORDER_SIZE,y+BORDER_SIZE};
	sfRectangleShape_setPosition(frame, pos);
	sfRectangleShape_setPosition(progress, pos);

	sfVector2f size = {w-2*BORDER_SIZE, h-2*BORDER_SIZE};
	sfRectangleShape_setSize(frame, size);
	size.x *= p;
	sfRectangleShape_setSize(progress, size);

	sfRenderWindow_drawRectangleShape(g->render, progress, NULL);
	sfRenderWindow_drawRectangleShape(g->render, frame, NULL);
}

void draw_building(graphics_t* g, building_t* b)
{
	int id = b->t->sprite;
	sfSprite* sprite = g->sprites[id];

	sfIntRect rect = sfSprite_getTextureRect(sprite);
	sfVector2f pos = {b->o.x - rect.width/2, b->o.y - rect.height};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);

	draw_progressBar(g, b->o.x - rect.width/2, b->o.y+1, rect.width, 5, b->buildProgress);
}

void draw_world(graphics_t* g, world_t* w)
{
	for (size_t i = 0; i < w->n_buildings; i++)
		draw_building(g, w->buildings[i]);

	for (size_t i = 0; i < 10; i++)
		draw_mine(g, &w->mines[i]);

	draw_character(g, &w->characters[0]);
}

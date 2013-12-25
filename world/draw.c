#include "draw.h"

#include <stdlib.h>
#include <math.h>

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

	int step = floor(c->step);
	if (step >= 3)
		step = 1;

	sfIntRect rect = {24*step, 32*c->dir, 24, 32};
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

void draw_building(graphics_t* g, building_t* b)
{
	int step;
	int n = b->t->n_sprites;
	float p = b->build_progress;

	if (n <= 1)
		step = 0;
	else
		step = floor((float)(n-2) * p);

	sfSprite* sprite = g->sprites[b->t->sprite];
	sfIntRect rect = {0, b->o.h*step, b->o.w, b->o.h};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &b->o, sprite);

	graphics_drawProgressBar(g, b->o.x - b->o.w/2, b->o.y+1, b->o.w, 5, b->build_progress);
}

void draw_world(graphics_t* g, world_t* w)
{
	for (int i = 0; i < w->n_buildings; i++)
		draw_building(g, w->buildings[i]);

	for (int i = 0; i < w->n_mines; i++)
		draw_mine(g, &w->mines[i]);

	for (int i = 0; i < w->n_characters; i++)
		draw_character(g, &w->characters[i]);
}
